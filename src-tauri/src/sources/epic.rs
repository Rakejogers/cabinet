//! Epic Games Store: reads the launcher's `.item` install manifests and its
//! local catalog cache (for cover art), launches via the launcher URI handler.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use base64::Engine;
use serde::Deserialize;

use super::{GameSource, SyncContext, epic_api};
use crate::covers::{ArtImage, ArtKind};
use crate::model::{Game, OwnedGame, StorePlaytime};
use crate::{net, platform};

pub struct EpicSource;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemManifest {
    display_name: String,
    app_name: String,
    install_location: String,
    catalog_namespace: String,
    catalog_item_id: String,
    #[serde(default)]
    app_categories: Vec<String>,
    #[serde(default)]
    main_game_app_name: String,
    #[serde(default, rename = "bIsIncompleteInstall")]
    is_incomplete_install: bool,
    #[serde(default)]
    install_size: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogItem {
    id: String,
    #[serde(default)]
    key_images: Vec<KeyImage>,
}

#[derive(Deserialize, Clone)]
struct KeyImage {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: String,
}

/// Preferred key-image types per artwork kind.
const COVER_TYPES: &[&str] = &["DieselGameBoxTall", "OfferImageTall", "Thumbnail", "DieselGameBox", "OfferImageWide"];
const HERO_TYPES: &[&str] = &["DieselGameBox", "OfferImageWide", "DieselStoreFrontWide"];
const LOGO_TYPES: &[&str] = &["DieselGameBoxLogo", "ProductLogo"];

impl GameSource for EpicSource {
    fn key(&self) -> &'static str {
        "epic"
    }

    fn name(&self) -> &'static str {
        "Epic Games"
    }

    fn scan(&self) -> Vec<Game> {
        let Some(dir) = platform::epic_manifests_dir() else {
            eprintln!("[epic] Epic Games Launcher manifests not found");
            return Vec::new();
        };
        let catalog = load_catalog(&dir);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut games = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "item") {
                continue;
            }
            match read_item(&path) {
                Ok(item) => {
                    if let Some(game) = to_game(item, &catalog) {
                        games.push(game);
                    }
                }
                Err(e) => eprintln!("[epic] {}: {e:#}", path.display()),
            }
        }
        games
    }

    fn launch(&self, game: &Game) -> Result<()> {
        platform::open_uri(&launcher_uri(game, "launch"))
    }

    fn install(&self, game: &Game) -> Result<()> {
        platform::open_uri(&launcher_uri(game, "install"))
    }

    fn owned(&self, ctx: &SyncContext) -> Result<Option<Vec<OwnedGame>>> {
        let Some(access) = ctx.accounts.epic_access()? else {
            return Ok(None);
        };
        let items = epic_api::library_items(&access.token)?;
        let playtime = epic_api::playtime(&access.token, &access.account_id).unwrap_or_else(|e| {
            eprintln!("[epic] playtime unavailable: {e:#}");
            HashMap::new()
        });

        // Unreal Engine marketplace assets live in the "ue" namespace.
        let items: Vec<_> = items.into_iter().filter(|i| i.namespace != "ue").collect();
        // Catalog lookups are one request per item, so only fetch unknown ones.
        let unknown: Vec<_> = items.iter().filter(|i| !ctx.known.contains_key(&i.app_name)).cloned().collect();
        let fetched = fetch_catalog(&access.token, &unknown);

        let mut owned = Vec::new();
        for item in &items {
            let (title, meta) = if let Some(game) = ctx.known.get(&item.app_name) {
                (game.title.clone(), game.meta.clone())
            } else if let Some(cat) = fetched.get(&item.catalog_item_id) {
                if !cat.is_game() {
                    continue;
                }
                let images: Vec<KeyImage> =
                    cat.key_images.iter().map(|k| KeyImage { kind: k.kind.clone(), url: k.url.clone() }).collect();
                (cat.title.clone(), game_meta(&item.namespace, &item.catalog_item_id, &images, None))
            } else {
                continue;
            };
            owned.push(OwnedGame {
                source: "epic".into(),
                source_id: item.app_name.clone(),
                title,
                meta,
                store_playtime: playtime.get(&item.app_name).filter(|secs| **secs > 0).map(|secs| StorePlaytime {
                    secs: *secs,
                    recent_secs: 0,
                    last_played: None,
                }),
            });
        }
        Ok(Some(owned))
    }

    fn fetch_art(&self, game: &Game, kind: ArtKind) -> Option<ArtImage> {
        // Epic's CDN resizes on the fly; full-size key art is often several MB.
        let (key, resize) = match kind {
            ArtKind::Cover => ("coverUrls", Some("?h=900&w=675&resize=1")),
            ArtKind::Hero => ("heroUrls", Some("?h=900&w=1600&resize=1")),
            ArtKind::Logo => ("logoUrls", None),
        };
        let urls = game.meta.get(key)?.as_array()?;
        urls.iter()
            .filter_map(|u| u.as_str())
            .find_map(|url| resize.and_then(|r| net::download(&format!("{url}{r}"))).or_else(|| net::download(url)))
    }
}

/// `com.epicgames.launcher://apps/<namespace>:<catalog id>:<app>?action=...`
fn launcher_uri(game: &Game, action: &str) -> String {
    let field = |k: &str| game.meta.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action={action}&silent=true",
        field("namespace"),
        field("catalogItemId"),
        game.source_id
    )
}

/// The metadata every Epic game carries, whether found installed or owned.
fn game_meta(namespace: &str, catalog_id: &str, images: &[KeyImage], size: Option<u64>) -> serde_json::Value {
    let urls_for = |types: &[&str]| -> Vec<String> {
        types
            .iter()
            .flat_map(|t| images.iter().filter(move |i| i.kind == *t && !i.url.is_empty()))
            .map(|i| i.url.clone())
            .collect()
    };
    serde_json::json!({
        "namespace": namespace,
        "catalogItemId": catalog_id,
        "coverUrls": urls_for(COVER_TYPES),
        "heroUrls": urls_for(HERO_TYPES),
        "logoUrls": urls_for(LOGO_TYPES),
        "sizeBytes": size,
    })
}

/// Catalog details for library items, fetched a few at a time.
fn fetch_catalog(token: &str, items: &[epic_api::LibraryItem]) -> HashMap<String, epic_api::CatalogItem> {
    if items.is_empty() {
        return HashMap::new();
    }
    let chunk = items.len().div_ceil(4);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|item| {
                            match epic_api::catalog_item(token, &item.namespace, &item.catalog_item_id) {
                                Ok(Some(cat)) => Some((item.catalog_item_id.clone(), cat)),
                                Ok(None) => None,
                                Err(e) => {
                                    eprintln!("[epic] catalog lookup for {} failed: {e:#}", item.app_name);
                                    None
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).flatten().collect()
    })
}

fn read_item(path: &Path) -> Result<ItemManifest> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

fn to_game(item: ItemManifest, catalog: &HashMap<String, Vec<KeyImage>>) -> Option<Game> {
    let is_game = item.app_categories.is_empty() || item.app_categories.iter().any(|c| c == "games");
    let is_dlc = !item.main_game_app_name.is_empty();
    if item.is_incomplete_install || !is_game || is_dlc {
        return None;
    }
    let images = catalog.get(&item.catalog_item_id).map(Vec::as_slice).unwrap_or_default();
    Some(Game {
        source: "epic".into(),
        source_id: item.app_name,
        title: item.display_name,
        install_dir: PathBuf::from(item.install_location),
        launch_target: String::new(),
        meta: game_meta(&item.catalog_namespace, &item.catalog_item_id, images, Some(item.install_size)),
        // Epic only keeps playtime server-side.
        store_playtime: None,
    })
}

/// The launcher keeps a base64-encoded JSON copy of the catalog entries for
/// owned items next to the manifests (`Data/Catalog/catcache.bin`).
fn load_catalog(manifests_dir: &Path) -> HashMap<String, Vec<KeyImage>> {
    let Some(path) = manifests_dir.parent().map(|d| d.join("Catalog").join("catcache.bin")) else {
        return HashMap::new();
    };
    let parsed = fs::read(&path)
        .map_err(anyhow::Error::from)
        .and_then(|raw| {
            let raw: Vec<u8> = raw.into_iter().filter(|b| !b.is_ascii_whitespace()).collect();
            Ok(base64::engine::general_purpose::STANDARD.decode(raw)?)
        })
        .and_then(|json| Ok(serde_json::from_slice::<Vec<CatalogItem>>(&json)?));
    match parsed {
        Ok(items) => items.into_iter().map(|i| (i.id, i.key_images)).collect(),
        Err(e) => {
            eprintln!("[epic] catalog cache unavailable ({}): {e:#}", path.display());
            HashMap::new()
        }
    }
}
