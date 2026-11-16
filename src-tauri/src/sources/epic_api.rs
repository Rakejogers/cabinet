//! Epic Games web services: sign-in, owned library, catalog metadata and
//! playtime.
//!
//! Epic has no public API for this. Like Legendary, Heroic and Playnite, this
//! signs in as the Epic Games Launcher's own OAuth client: the user logs in on
//! epicgames.com and pastes back a one-time authorization code. It is
//! unofficial and may stop working if Epic changes these services.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use base64::Engine;
use serde::Deserialize;

use crate::net;

/// The Epic Games Launcher's public OAuth client (as used by Legendary).
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";

const OAUTH_URL: &str = "https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token";
const LIBRARY_URL: &str = "https://library-service.live.use1a.on.epicgames.com/library/api/public";
const CATALOG_URL: &str = "https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/namespace";

/// Where the user signs in; Epic then shows a JSON page with `authorizationCode`.
pub fn login_url() -> String {
    let redirect = format!("https://www.epicgames.com/id/api/redirect?clientId={CLIENT_ID}&responseType=code");
    format!("https://www.epicgames.com/id/login?redirectUrl={}", net::url_encode(&redirect))
}

#[derive(Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub expires_in: i64,
}

fn basic_auth() -> String {
    let creds = base64::engine::general_purpose::STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"));
    format!("basic {creds}")
}

fn token_request(form: &[(&str, &str)]) -> Result<Tokens> {
    let json = net::post_form(OAUTH_URL, &basic_auth(), form)?;
    serde_json::from_value(json).context("unexpected Epic token response")
}

/// Accepts either the bare code or the whole JSON blob Epic's page shows.
pub fn parse_code(input: &str) -> Option<String> {
    let input = input.trim();
    if input.starts_with('{') {
        let v: serde_json::Value = serde_json::from_str(input).ok()?;
        return v.get("authorizationCode")?.as_str().map(str::to_string);
    }
    let code: String = input.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    (code.len() >= 16).then_some(code)
}

pub fn exchange_code(input: &str) -> Result<Tokens> {
    let Some(code) = parse_code(input) else {
        bail!("That doesn't look like an Epic authorization code. Copy the value of \"authorizationCode\".");
    };
    token_request(&[("grant_type", "authorization_code"), ("code", &code), ("token_type", "eg1")])
        .context("Epic sign-in failed (codes expire after a few minutes, so get a fresh one)")
}

pub fn refresh(refresh_token: &str) -> Result<Tokens> {
    token_request(&[("grant_type", "refresh_token"), ("refresh_token", refresh_token), ("token_type", "eg1")])
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub namespace: String,
    pub catalog_item_id: String,
    pub app_name: String,
}

/// Every entitlement in the account's library (games, but also DLC and assets).
pub fn library_items(token: &str) -> Result<Vec<LibraryItem>> {
    let auth = format!("bearer {token}");
    let mut items = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..100 {
        let mut url = format!("{LIBRARY_URL}/items?includeMetadata=true");
        if let Some(c) = &cursor {
            url.push_str(&format!("&cursor={}", net::url_encode(c)));
        }
        let page = net::get_json_auth(&url, Some(&auth))?;
        if let Some(records) = page.get("records").and_then(|r| r.as_array()) {
            items.extend(records.iter().filter_map(|r| serde_json::from_value(r.clone()).ok()));
        }
        cursor = page.pointer("/responseMetadata/nextCursor").and_then(|c| c.as_str()).map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }
    Ok(items)
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub title: String,
    #[serde(default)]
    pub key_images: Vec<KeyImage>,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub main_game_item: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyImage {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Category {
    #[serde(default)]
    pub path: String,
}

impl CatalogItem {
    /// Base games only: no DLC, add-ons, engine assets or other software.
    pub fn is_game(&self) -> bool {
        self.main_game_item.is_none() && self.categories.iter().any(|c| c.path == "games")
    }
}

pub fn catalog_item(token: &str, namespace: &str, id: &str) -> Result<Option<CatalogItem>> {
    let url = format!(
        "{CATALOG_URL}/{namespace}/bulk/items?id={id}&includeDLCDetails=true&includeMainGameDetails=true&country=US&locale=en-US"
    );
    let json = net::get_json_auth(&url, Some(&format!("bearer {token}")))?;
    Ok(json.get(id).and_then(|v| serde_json::from_value(v.clone()).ok()))
}

/// Total playtime in seconds per AppName, as tracked by Epic.
pub fn playtime(token: &str, account_id: &str) -> Result<HashMap<String, i64>> {
    let url = format!("{LIBRARY_URL}/playtime/account/{account_id}/all");
    let json = net::get_json_auth(&url, Some(&format!("bearer {token}")))?;
    Ok(json
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r.get("artifactId")?.as_str()?.to_string(), r.get("totalTime")?.as_i64()?)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::parse_code;

    #[test]
    fn parses_codes() {
        let json =
            r#"{"redirectUrl":"https://localhost","authorizationCode":"0123456789abcdef0123456789abcdef","sid":null}"#;
        assert_eq!(parse_code(json).as_deref(), Some("0123456789abcdef0123456789abcdef"));
        assert_eq!(
            parse_code("  0123456789abcdef0123456789abcdef \n").as_deref(),
            Some("0123456789abcdef0123456789abcdef")
        );
        assert_eq!(parse_code("nope"), None);
    }
}
