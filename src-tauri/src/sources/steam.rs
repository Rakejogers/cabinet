//! Steam: reads `libraryfolders.vdf` and every `appmanifest_*.acf` in each
//! library, launches through the `steam://` URI handler.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{GameSource, SyncContext, vdf};
use crate::covers::{self, ArtImage, ArtKind};
use crate::model::{Game, OwnedGame, StorePlaytime};
use crate::{net, platform};

/// Tools and runtimes Steam installs alongside games.
const IGNORED_APP_IDS: &[&str] = &[
    "228980",  // Steamworks Common Redistributables
    "250820",  // SteamVR
    "1070560", // Steam Linux Runtime 1.0 (scout)
    "1391110", // Steam Linux Runtime 2.0 (soldier)
    "1628350", // Steam Linux Runtime 3.0 (sniper)
    "1493710", // Proton Experimental
    "2180100", // Proton Hotfix
    "1826330", // Proton EasyAntiCheat Runtime
    "1161040", // Proton BattlEye Runtime
];
const IGNORED_NAME_PREFIXES: &[&str] = &["Proton ", "Steam Linux Runtime", "Steamworks "];

/// `StateFlags` bit meaning the app is fully installed.
const STATE_FULLY_INSTALLED: u32 = 4;

const ASSET_CDN: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";

/// Offset between a 64-bit SteamID and the 32-bit account id used for `userdata/<id>`.
const STEAMID64_BASE: u64 = 76_561_197_960_265_728;

pub struct SteamSource;

impl GameSource for SteamSource {
    fn key(&self) -> &'static str {
        "steam"
    }

    fn name(&self) -> &'static str {
        "Steam"
    }

    fn scan(&self) -> Vec<Game> {
        let Some(root) = platform::steam_root() else {
            eprintln!("[steam] Steam installation not found");
            return Vec::new();
        };
        let playtime = read_playtime(&root);
        let mut seen = HashSet::new();
        let mut games = Vec::new();
        for library in library_folders(&root) {
            let steamapps = library.join("steamapps");
            let Ok(entries) = fs::read_dir(&steamapps) else {
                eprintln!("[steam] library not readable: {}", steamapps.display());
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                match parse_manifest(&entry.path(), &steamapps) {
                    Ok(Some(mut game)) => {
                        if seen.insert(game.source_id.clone()) {
                            game.store_playtime = playtime.get(&game.source_id).copied();
                            games.push(game);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[steam] {}: {e:#}", entry.path().display()),
                }
            }
        }
        games
    }

    fn launch(&self, game: &Game) -> Result<()> {
        platform::open_uri(&format!("steam://rungameid/{}", game.source_id))
    }

    fn install(&self, game: &Game) -> Result<()> {
        platform::open_uri(&format!("steam://install/{}", game.source_id))
    }

    fn store_page(&self, game: &Game) -> Option<String> {
        Some(format!("https://store.steampowered.com/app/{}/", game.source_id))
    }

    /// Uses the official Web API with the user's own key.
    fn owned(&self, ctx: &SyncContext) -> Result<Option<Vec<OwnedGame>>> {
        let Some(key) = ctx.accounts.steam_api_key() else {
            return Ok(None);
        };
        let root = platform::steam_root().context("Steam is not installed")?;
        let account = current_account_id(&root).context("no signed-in Steam account found")?;
        let url = format!(
            "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&include_played_free_games=1&format=json",
            net::url_encode(&key),
            account + STEAMID64_BASE
        );
        let json = match net::get_json_auth(&url, None) {
            Ok(json) => json,
            Err(e) if matches!(net::status_of(&e), Some(401 | 403)) => {
                anyhow::bail!("Steam didn't accept that API key. Check it at steamcommunity.com/dev/apikey.")
            }
            Err(e) => return Err(e.context("couldn't fetch your Steam library")),
        };
        let games = json.pointer("/response/games").and_then(|g| g.as_array()).cloned().unwrap_or_default();
        let owned = games
            .iter()
            .filter_map(|g| {
                let app_id = g.get("appid")?.as_u64()?.to_string();
                let name = g.get("name")?.as_str()?.to_string();
                if IGNORED_APP_IDS.contains(&app_id.as_str())
                    || IGNORED_NAME_PREFIXES.iter().any(|p| name.starts_with(p))
                {
                    return None;
                }
                let minutes = g.get("playtime_forever").and_then(|v| v.as_i64()).unwrap_or(0);
                let recent = g.get("playtime_2weeks").and_then(|v| v.as_i64()).unwrap_or(0);
                let last = g.get("rtime_last_played").and_then(|v| v.as_i64()).filter(|t| *t > 0);
                Some(OwnedGame {
                    source: "steam".into(),
                    source_id: app_id,
                    title: name,
                    meta: serde_json::json!({}),
                    store_playtime: (minutes > 0).then_some(StorePlaytime {
                        secs: minutes * 60,
                        recent_secs: recent * 60,
                        last_played: last,
                    }),
                })
            })
            .collect();
        Ok(Some(owned))
    }

    fn fetch_art(&self, game: &Game, kind: ArtKind) -> Option<ArtImage> {
        fetch_art_by_id(&game.source_id, kind)
    }
}

/// Order of preference: Steam's own library cache (everything the client has
/// shown), the un-hashed CDN path (most older titles), then the public store
/// API, which knows the hashed asset paths newer titles use.
pub fn fetch_art_by_id(app_id: &str, kind: ArtKind) -> Option<ArtImage> {
    {
        let cache = platform::steam_root().map(|r| r.join("appcache").join("librarycache"));
        let local = |names: &[&str]| {
            cache.as_ref().and_then(|c| find_cached(c, app_id, names)).and_then(|p| covers::from_file(&p))
        };
        let cdn = |file: &str| net::download(&format!("{ASSET_CDN}steam/apps/{app_id}/{file}"));
        match kind {
            ArtKind::Cover => local(&["library_600x900_2x.jpg", "library_600x900.jpg"])
                .or_else(|| cdn("library_600x900_2x.jpg"))
                .or_else(|| store_api_asset(app_id, &["library_capsule_2x", "library_capsule", "header"]))
                .or_else(|| local(&["header.jpg"])),
            ArtKind::Hero => local(&["library_hero.jpg"])
                .or_else(|| cdn("library_hero.jpg"))
                .or_else(|| store_api_asset(app_id, &["library_hero"])),
            ArtKind::Logo => local(&["logo.png"]).or_else(|| cdn("logo.png")),
        }
    }
}

/// Lowercase alphanumerics only, so "Steep™" matches "Steep".
fn normalize_title(title: &str) -> String {
    title
        .chars()
        .filter(|c| !matches!(c, '™' | '®' | '©'))
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Finds a Steam app whose name matches `title`, retrying with trailing words
/// dropped ("Rainbow Six Siege X" -> "Rainbow Six Siege").
pub fn find_app_by_title(title: &str) -> Option<u64> {
    let words: Vec<&str> = title.split_whitespace().collect();
    for len in (2.min(words.len())..=words.len()).rev() {
        let query = words[..len].join(" ");
        let wanted = normalize_title(&query);
        let url =
            format!("https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US", net::url_encode(&query));
        let json = net::get_json(&url)?;
        let hit = json.get("items")?.as_array()?.iter().find_map(|item| {
            let name = item.get("name")?.as_str()?;
            (normalize_title(name) == wanted).then(|| item.get("id")?.as_u64()).flatten()
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

/// Every library folder Steam knows about, the Steam root first.
fn library_folders(root: &Path) -> Vec<PathBuf> {
    let mut folders = vec![root.to_path_buf()];
    let file = root.join("steamapps").join("libraryfolders.vdf");
    match fs::read_to_string(&file).map_err(anyhow::Error::from).and_then(|t| vdf::parse(&t)) {
        Ok(doc) => {
            if let Some(list) = doc.get("libraryfolders") {
                for (_, folder) in list.entries() {
                    if let Some(path) = folder.str("path") {
                        folders.push(PathBuf::from(path));
                    }
                }
            }
        }
        Err(e) => eprintln!("[steam] {}: {e:#}", file.display()),
    }
    let mut seen = HashSet::new();
    folders.retain(|f| seen.insert(platform::normalize_path(f)));
    folders
}

fn parse_manifest(path: &Path, steamapps: &Path) -> Result<Option<Game>> {
    let doc = vdf::parse(&fs::read_to_string(path)?)?;
    let Some(state) = doc.get("AppState") else {
        anyhow::bail!("missing AppState");
    };
    let (Some(app_id), Some(name), Some(install_dir)) =
        (state.str("appid"), state.str("name"), state.str("installdir"))
    else {
        anyhow::bail!("manifest is missing appid, name or installdir");
    };
    let flags: u32 = state.str("StateFlags").and_then(|f| f.parse().ok()).unwrap_or(0);
    if flags & STATE_FULLY_INSTALLED == 0
        || IGNORED_APP_IDS.contains(&app_id)
        || IGNORED_NAME_PREFIXES.iter().any(|p| name.starts_with(p))
    {
        return Ok(None);
    }
    Ok(Some(Game {
        source: "steam".into(),
        source_id: app_id.to_string(),
        title: name.to_string(),
        install_dir: steamapps.join("common").join(install_dir),
        launch_target: format!("steam://rungameid/{app_id}"),
        meta: serde_json::json!({
            "sizeBytes": state.str("SizeOnDisk").and_then(|s| s.parse::<u64>().ok()),
        }),
        store_playtime: None,
    }))
}

/// The account Steam last signed in with: its `userdata` folder id and
/// display name.
fn current_user(root: &Path) -> Option<(u64, String)> {
    let text = fs::read_to_string(root.join("config").join("loginusers.vdf")).ok()?;
    let doc = vdf::parse(&text).ok()?;
    let num = |v: &vdf::Vdf, k: &str| v.str(k).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let (steam_id, user) =
        doc.get("users")?.entries().iter().max_by_key(|(_, user)| (num(user, "MostRecent"), num(user, "Timestamp")))?;
    let id = steam_id.parse::<u64>().ok()?.checked_sub(STEAMID64_BASE)?;
    Some((id, user.str("PersonaName").unwrap_or_default().to_string()))
}

fn current_account_id(root: &Path) -> Option<u64> {
    current_user(root).map(|(id, _)| id)
}

/// Display name of the signed-in Steam account.
pub fn account_name() -> Option<String> {
    current_user(&platform::steam_root()?).map(|(_, name)| name).filter(|n| !n.is_empty())
}

/// Per-app playtime from the current user's `localconfig.vdf`, keyed by app id.
fn read_playtime(root: &Path) -> HashMap<String, StorePlaytime> {
    let mut out = HashMap::new();
    let Some(account) = current_account_id(root) else {
        eprintln!("[steam] no signed-in account found; skipping playtime import");
        return out;
    };
    let path = root.join("userdata").join(account.to_string()).join("config").join("localconfig.vdf");
    let doc = match fs::read_to_string(&path).map_err(anyhow::Error::from).and_then(|t| vdf::parse(&t)) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("[steam] {}: {e:#}", path.display());
            return out;
        }
    };
    let apps =
        ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"].iter().try_fold(&doc, |node, key| node.get(key));
    for (app_id, app) in apps.map(|a| a.entries()).unwrap_or_default() {
        let minutes = app.str("Playtime").and_then(|m| m.parse::<i64>().ok());
        if let Some(minutes) = minutes.filter(|m| *m > 0) {
            let last_played = app.str("LastPlayed").and_then(|t| t.parse::<i64>().ok()).filter(|t| *t > 0);
            let recent_minutes = app.str("Playtime2wks").and_then(|m| m.parse::<i64>().ok()).unwrap_or(0);
            out.insert(
                app_id.clone(),
                StorePlaytime { secs: minutes * 60, recent_secs: recent_minutes * 60, last_played },
            );
        }
    }
    out
}

/// Looks for one of `names` in the Steam library cache. Current clients store
/// art in `librarycache/<appid>/[<hash>/]<name>`, older ones in
/// `librarycache/<appid>_<name>`.
fn find_cached(cache: &Path, app_id: &str, names: &[&str]) -> Option<PathBuf> {
    let dir = cache.join(app_id);
    for name in names {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
    }
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                for name in names {
                    let nested = entry.path().join(name);
                    if nested.is_file() {
                        return Some(nested);
                    }
                }
            }
        }
    }
    for name in names {
        let legacy = cache.join(format!("{app_id}_{name}"));
        if legacy.is_file() {
            return Some(legacy);
        }
    }
    None
}

/// Downloads the first of `keys` that the store asset API lists for the app.
fn store_api_asset(app_id: &str, keys: &[&str]) -> Option<ArtImage> {
    let input = serde_json::json!({
        "ids": [{ "appid": app_id.parse::<u64>().ok()? }],
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_assets": true }
    });
    let url = format!(
        "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}",
        net::url_encode(&input.to_string())
    );
    let response = net::get_json(&url)?;
    let assets = response.pointer("/response/store_items/0/assets")?;
    let format = assets.get("asset_url_format")?.as_str()?;
    keys.iter()
        .filter_map(|key| assets.get(*key)?.as_str())
        .find_map(|file| net::download(&format!("{ASSET_CDN}{}", format.replace("${FILENAME}", file))))
}
