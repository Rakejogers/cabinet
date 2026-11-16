//! Xbox app / PC Game Pass: GDK games live in the Xbox library folders (found
//! via `platform`) with a `MicrosoftGame.config` describing them. They launch
//! through the shell as packaged apps, and artwork comes from the public
//! Microsoft Store catalog.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};

use super::GameSource;
use crate::covers::{self, ArtImage, ArtKind};
use crate::model::Game;
use crate::{net, platform};

pub struct XboxSource;

impl GameSource for XboxSource {
    fn key(&self) -> &'static str {
        "xbox"
    }

    fn name(&self) -> &'static str {
        "Xbox"
    }

    fn scan(&self) -> Vec<Game> {
        let mut games = Vec::new();
        for root in platform::xbox_library_roots() {
            let Ok(entries) = fs::read_dir(&root) else { continue };
            for entry in entries.flatten() {
                let content = entry.path().join("Content");
                match read_game(&content) {
                    Some(game) => games.push(game),
                    None => {
                        if content.is_dir() {
                            eprintln!("[xbox] no usable game config in {}", content.display());
                        }
                    }
                }
            }
        }
        games
    }

    fn launch(&self, game: &Game) -> Result<()> {
        let aumid = game.meta.get("aumid").and_then(|v| v.as_str()).unwrap_or_default();
        platform::open_uri(&format!(r"shell:AppsFolder\{aumid}"))
    }

    fn store_page(&self, game: &Game) -> Option<String> {
        let store_id = game.meta.get("storeId")?.as_str()?;
        Some(format!("ms-windows-store://pdp/?productid={store_id}"))
    }

    fn fetch_art(&self, game: &Game, kind: ArtKind) -> Option<ArtImage> {
        let from_store = game.meta.get("storeId").and_then(|v| v.as_str()).and_then(|id| store_image(id, kind));
        from_store.or_else(|| {
            // Fall back to the art shipped inside the package.
            let file = match kind {
                ArtKind::Cover => "squareLogo",
                ArtKind::Hero => "splash",
                ArtKind::Logo => return None,
            };
            let name = game.meta.get(file)?.as_str()?;
            covers::from_file(&game.install_dir.join(name))
        })
    }
}

/// Finds `MicrosoftGame.config` regardless of its casing.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Position of the first `<tag` element, not a longer name that starts the
/// same way (`<Executable` must not match `<ExecutableList>`).
fn find_tag(xml: &str, tag: &str) -> Option<usize> {
    let open = format!("<{tag}");
    let mut from = 0;
    while let Some(pos) = xml[from..].find(&open) {
        let start = from + pos;
        let next = xml[start + open.len()..].chars().next();
        if next.is_some_and(|c| c.is_whitespace() || c == '>' || c == '/') {
            return Some(start);
        }
        from = start + open.len();
    }
    None
}

/// Value of `attr="..."` on the first `<tag ...>` element. The configs are
/// simple, machine-written XML, so a full parser isn't needed.
fn xml_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let start = find_tag(xml, tag)?;
    let element = &xml[start..start + xml[start..].find('>')?];
    let key = format!("{attr}=\"");
    let value_start = element.find(&key)? + key.len();
    let value = &element[value_start..value_start + element[value_start..].find('"')?];
    Some(
        value
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">"),
    )
}

fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&format!("</{tag}>"))?;
    Some(xml[start..end].trim().to_string())
}

fn read_game(content: &Path) -> Option<Game> {
    let config = fs::read_to_string(find_file(content, "MicrosoftGame.config")?).ok()?;
    let manifest = find_file(content, "appxmanifest.xml").and_then(|p| fs::read_to_string(p).ok());

    let name = xml_attr(&config, "Identity", "Name")?;
    let publisher = xml_attr(&config, "Identity", "Publisher")?;
    // The manifest's Application Id is what the shell uses; the config's
    // executable id is the fallback.
    let app_id = manifest
        .as_deref()
        .and_then(|m| xml_attr(m, "Application", "Id"))
        .or_else(|| xml_attr(&config, "Executable", "Id"))?;
    let aumid = format!("{name}_{}!{app_id}", publisher_id(&publisher));

    let folder = content.parent()?.file_name()?.to_string_lossy().to_string();
    let title = xml_attr(&config, "ShellVisuals", "DefaultDisplayName")
        .filter(|t| !t.starts_with("ms-resource:"))
        .unwrap_or(folder);

    Some(Game {
        source: "xbox".into(),
        source_id: name,
        title,
        install_dir: content.to_path_buf(),
        launch_target: String::new(),
        meta: serde_json::json!({
            "aumid": aumid,
            "storeId": xml_text(&config, "StoreId"),
            "splash": xml_attr(&config, "ShellVisuals", "SplashScreenImage"),
            "squareLogo": xml_attr(&config, "ShellVisuals", "Square480x480Logo"),
        }),
        store_playtime: None,
    })
}

/// The publisher hash in package family names: SHA-256 of the UTF-16LE
/// publisher string, first 8 bytes, written as 13 Crockford base32 digits.
fn publisher_id(publisher: &str) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let utf16: Vec<u8> = publisher.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let hash = Sha256::digest(&utf16);
    // 64 bits padded to 65 so they split evenly into 5-bit groups.
    let bits = (u128::from(u64::from_be_bytes(hash[..8].try_into().unwrap()))) << 1;
    (0..13).rev().map(|i| ALPHABET[((bits >> (i * 5)) & 0x1f) as usize] as char).collect()
}

/// Artwork from the Microsoft Store catalog (public, no sign-in).
fn store_image(store_id: &str, kind: ArtKind) -> Option<ArtImage> {
    let url =
        format!("https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={store_id}&market=US&languages=en-us");
    let json = net::get_json(&url)?;
    let images = json.pointer("/Products/0/LocalizedProperties/0/Images")?.as_array()?;
    let purposes: &[&str] = match kind {
        ArtKind::Cover => &["Poster", "BoxArt"],
        ArtKind::Hero => &["SuperHeroArt", "TitledHeroArt"],
        ArtKind::Logo => return None,
    };
    purposes.iter().find_map(|purpose| {
        let uri = images
            .iter()
            .find(|i| i.get("ImagePurpose").and_then(|p| p.as_str()) == Some(purpose))?
            .get("Uri")?
            .as_str()?;
        let uri = if uri.starts_with("//") { format!("https:{uri}") } else { uri.to_string() };
        net::download(&uri)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_publisher_ids() {
        assert_eq!(
            publisher_id("CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"),
            "8wekyb3d8bbwe"
        );
    }

    #[test]
    fn reads_xml_attributes() {
        let xml = r#"<Game><Identity Name="Microsoft.MinecraftUWP"
            Publisher="CN=A &amp; B" Version="1.0" /><StoreId>9NBLGGH2JHXJ</StoreId></Game>"#;
        assert_eq!(xml_attr(xml, "Identity", "Name").as_deref(), Some("Microsoft.MinecraftUWP"));
        assert_eq!(xml_attr(xml, "Identity", "Publisher").as_deref(), Some("CN=A & B"));
        assert_eq!(xml_text(xml, "StoreId").as_deref(), Some("9NBLGGH2JHXJ"));
    }

    #[test]
    fn skips_longer_tag_names() {
        let xml = r#"<ExecutableList><Executable Name="Game.exe" Id="Game" /></ExecutableList>"#;
        assert_eq!(xml_attr(xml, "Executable", "Id").as_deref(), Some("Game"));
    }
}
