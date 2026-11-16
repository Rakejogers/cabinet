//! HTTP helpers shared by the store integrations.

use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::covers::{self, ArtImage};

const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .user_agent("Cabinet/0.1")
            // Statuses are checked by hand so API error bodies can be reported.
            .http_status_as_error(false)
            // Trust the OS certificate store, so machines behind TLS-inspecting
            // antivirus or corporate proxies work like the browser does.
            .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
            .build()
            .into()
    })
}

fn read(mut response: ureq::http::Response<ureq::Body>, limit: u64) -> Result<Vec<u8>> {
    let status = response.status().as_u16();
    let body = response.body_mut().with_config().limit(limit).read_to_vec().unwrap_or_default();
    if !(200..300).contains(&status) {
        bail!(HttpError { status, message: error_message(&body) });
    }
    Ok(body)
}

/// A non-success HTTP response, with the most readable message the body had.
#[derive(Debug)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "HTTP {}", self.status)
        } else {
            write!(f, "HTTP {}: {}", self.status, self.message)
        }
    }
}

impl std::error::Error for HttpError {}

/// The HTTP status of an error from this module, if it was one.
pub fn status_of(err: &anyhow::Error) -> Option<u16> {
    err.chain().find_map(|e| e.downcast_ref::<HttpError>()).map(|e| e.status)
}

/// Pulls a human-readable message out of a JSON or HTML error body.
fn error_message(body: &[u8]) -> String {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
        for key in ["errorMessage", "message", "error_description", "error"] {
            if let Some(msg) = json.get(key).and_then(|v| v.as_str()) {
                return msg.to_string();
            }
        }
    }
    let text = String::from_utf8_lossy(body);
    let mut plain = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                plain.push(' ');
            }
            _ if !in_tag => plain.push(c),
            _ => {}
        }
    }
    plain.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(200).collect()
}

/// URLs can carry API keys in the query string; never log those.
fn strip_query(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// GET returning JSON, with an optional `Authorization` header.
pub fn get_json_auth(url: &str, auth: Option<&str>) -> Result<serde_json::Value> {
    let mut req = agent().get(url);
    if let Some(auth) = auth {
        req = req.header("Authorization", auth);
    }
    let response = req.call().with_context(|| format!("request to {} failed", strip_query(url)))?;
    let body = read(response, MAX_JSON_BYTES)?;
    serde_json::from_slice(&body).with_context(|| format!("{} returned invalid JSON", strip_query(url)))
}

/// POST an urlencoded form, returning JSON.
pub fn post_form(url: &str, auth: &str, form: &[(&str, &str)]) -> Result<serde_json::Value> {
    let response = agent()
        .post(url)
        .header("Authorization", auth)
        .send_form(form.iter().copied())
        .with_context(|| format!("request to {} failed", strip_query(url)))?;
    let body = read(response, MAX_JSON_BYTES)?;
    serde_json::from_slice(&body).with_context(|| format!("{} returned invalid JSON", strip_query(url)))
}

/// Best-effort JSON GET for optional lookups; failures are logged, not raised.
pub fn get_json(url: &str) -> Option<serde_json::Value> {
    get_json_auth(url, None).map_err(|e| eprintln!("[http] {}: {e:#}", strip_query(url))).ok()
}

/// Downloads an image, rejecting anything that isn't one. 404s are expected
/// while probing candidate URLs and aren't logged.
pub fn download(url: &str) -> Option<ArtImage> {
    let response = agent().get(url).call().map_err(|e| eprintln!("[http] {}: {e}", strip_query(url))).ok()?;
    if response.status().as_u16() == 404 {
        return None;
    }
    let bytes = read(response, MAX_IMAGE_BYTES).map_err(|e| eprintln!("[http] {}: {e:#}", strip_query(url))).ok()?;
    covers::from_bytes(bytes)
}

/// Percent-encodes everything outside the RFC 3986 unreserved set.
pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "needs network"]
    fn downloads_a_steam_cover() {
        let img = super::download(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/library_600x900_2x.jpg",
        );
        assert!(img.is_some());
    }
}
