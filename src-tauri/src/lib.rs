mod accounts;
mod covers;
mod db;
mod jobs;
mod model;
mod net;
mod platform;
mod sources;
mod themes;
mod tracker;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use accounts::Accounts;
use db::Db;
use model::{Game, LibraryGame, Session};
use sources::GameSource;
use tracker::Tracker;

pub(crate) struct AppState {
    db: Arc<Db>,
    sources: Arc<Vec<Box<dyn GameSource>>>,
    accounts: Accounts,
    tracker: Tracker,
    art_dir: PathBuf,
    themes_dir: PathBuf,
    art_worker_busy: Arc<AtomicBool>,
    owned_sync_busy: AtomicBool,
}

impl AppState {
    fn source(&self, key: &str) -> CmdResult<&dyn GameSource> {
        self.sources
            .iter()
            .find(|s| s.key() == key)
            .map(|s| s.as_ref())
            .ok_or_else(|| format!("No integration for {key}"))
    }

    fn game(&self, id: &str) -> CmdResult<(Game, bool)> {
        self.db.game(id).map_err(err)?.ok_or_else(|| "That game is no longer in the library.".to_string())
    }
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn err_chain(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn library_snapshot(state: &AppState) -> CmdResult<Vec<LibraryGame>> {
    let running = state.tracker.running_ids();
    let mut games = state.db.library().map_err(err)?;
    for g in &mut games {
        g.running = running.contains(&g.id);
    }
    Ok(games)
}

// ---- Library ----------------------------------------------------------------

#[tauri::command]
fn get_library(state: State<'_, AppState>) -> CmdResult<Vec<LibraryGame>> {
    library_snapshot(&state)
}

#[derive(Serialize)]
struct SourceInfo {
    key: &'static str,
    name: &'static str,
}

#[tauri::command]
fn get_sources(state: State<'_, AppState>) -> Vec<SourceInfo> {
    state.sources.iter().map(|s| SourceInfo { key: s.key(), name: s.name() }).collect()
}

/// Rescans every store. `manual` is set when the user asked for it, which also
/// retries artwork that couldn't be found earlier. Owned libraries from store
/// accounts sync afterwards in the background.
#[tauri::command]
async fn rescan(app: AppHandle, manual: bool) -> CmdResult<Vec<LibraryGame>> {
    let (db, sources) = {
        let state = app.state::<AppState>();
        (state.db.clone(), state.sources.clone())
    };
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        let started = Instant::now();
        let results: Vec<(&'static str, Vec<Game>)> = thread::scope(|scope| {
            let handles: Vec<_> = sources.iter().map(|s| scope.spawn(move || (s.key(), s.scan()))).collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        for (key, games) in &results {
            eprintln!("[scan] {key}: {} games", games.len());
            db.sync_source(key, games)?;
        }
        eprintln!("[scan] done in {:?}", started.elapsed());
        Ok(())
    })
    .await
    .map_err(err)?
    .map_err(err_chain)?;

    // Snapshot first: the art worker may emit updates before this response
    // arrives, and the UI must not overwrite them with older data.
    let library = library_snapshot(&app.state::<AppState>());
    jobs::spawn_art_worker(&app, manual);
    jobs::spawn_owned_sync(&app);
    library
}

/// Plays an installed game, or asks its store to install an owned one.
#[tauri::command]
fn launch_game(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let (game, installed) = state.game(&id)?;
    let source = state.source(&game.source)?;
    if !installed {
        return source.install(&game).map_err(err_chain);
    }
    source.launch(&game).map_err(err_chain)?;
    state.db.record_launch(&id).map_err(err)?;
    state.tracker.track(app, state.db.clone(), id, game.install_dir);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameDetails {
    game: LibraryGame,
    source_name: &'static str,
    source_id: String,
    install_dir: Option<String>,
    size_bytes: Option<u64>,
    store_page: Option<String>,
}

#[tauri::command]
fn game_details(state: State<'_, AppState>, id: String) -> CmdResult<GameDetails> {
    let (game, installed) = state.game(&id)?;
    let source = state.source(&game.source)?;
    let mut entry = state.db.library_game(&id).map_err(err)?.ok_or("Game not found")?;
    entry.running = state.tracker.running_ids().contains(&id);
    let install_dir = (installed && game.install_dir.is_dir()).then(|| game.install_dir.to_string_lossy().to_string());
    Ok(GameDetails {
        game: entry,
        source_name: source.name(),
        source_id: game.source_id.clone(),
        install_dir,
        size_bytes: game.meta.get("sizeBytes").and_then(|v| v.as_u64()).filter(|_| installed),
        store_page: source.store_page(&game),
    })
}

/// Play sessions that ended after `since` (unix seconds), for one game or all.
#[tauri::command]
fn get_sessions(state: State<'_, AppState>, game_id: Option<String>, since: i64) -> CmdResult<Vec<Session>> {
    state.db.sessions(game_id.as_deref(), since).map_err(err)
}

#[tauri::command]
fn open_install_dir(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let (game, _) = state.game(&id)?;
    if !game.install_dir.is_dir() {
        return Err("The install folder no longer exists.".into());
    }
    platform::open_uri(&game.install_dir.to_string_lossy()).map_err(err_chain)
}

#[tauri::command]
fn open_store_page(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let (game, _) = state.game(&id)?;
    let url = state.source(&game.source)?.store_page(&game).ok_or("This store has no page link.")?;
    platform::open_uri(&url).map_err(err_chain)
}

// ---- Accounts ----------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountStatus {
    source: &'static str,
    connected: bool,
    /// Who is signed in, when the store tells us.
    account: Option<String>,
    last_sync: Option<jobs::SyncInfo>,
}

#[tauri::command]
fn get_accounts(app: AppHandle, state: State<'_, AppState>) -> Vec<AccountStatus> {
    vec![
        AccountStatus {
            source: "steam",
            connected: state.accounts.steam_api_key().is_some(),
            account: sources::steam_account_name(),
            last_sync: jobs::last_sync(&app, "steam"),
        },
        AccountStatus {
            source: "epic",
            connected: state.accounts.epic_display_name().is_some(),
            account: state.accounts.epic_display_name(),
            last_sync: jobs::last_sync(&app, "epic"),
        },
    ]
}

/// Saves (or with `None`, removes) the Steam Web API key, then syncs.
#[tauri::command]
async fn set_steam_api_key(app: AppHandle, key: Option<String>) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let previous_key = state.accounts.steam_api_key();
        let previous_sync = state.db.get_setting("sync.steam").ok().flatten();
        state.accounts.set_steam_api_key(key.as_deref()).map_err(err_chain)?;
        if state.accounts.steam_api_key().is_some() {
            jobs::sync_owned(&app, Some("steam"));
            if let Some(e) = jobs::last_sync(&app, "steam").and_then(|s| s.error) {
                // Don't keep a key that doesn't work; put things back as they were.
                let _ = state.accounts.set_steam_api_key(previous_key.as_deref());
                let _ = state.db.set_setting("sync.steam", previous_sync.as_deref().unwrap_or("null"));
                return Err(e);
            }
        } else {
            state.db.clear_owned("steam").map_err(err)?;
            let _ = state.db.set_setting("sync.steam", "null");
        }
        Ok(())
    })
    .await
    .map_err(err)?
}

#[tauri::command]
fn epic_login_url() -> String {
    sources::epic_api::login_url()
}

#[tauri::command]
async fn epic_login(app: AppHandle, code: String) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let name = state.accounts.epic_login(&code).map_err(err_chain)?;
        jobs::sync_owned(&app, Some("epic"));
        Ok(name)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
fn epic_logout(state: State<'_, AppState>) -> CmdResult<()> {
    state.accounts.epic_logout().map_err(err_chain)?;
    state.db.clear_owned("epic").map_err(err)?;
    state.db.set_setting("sync.epic", "null").map_err(err)
}

#[tauri::command]
async fn sync_accounts(app: AppHandle) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || jobs::sync_owned(&app, None)).await.map_err(err)
}

// ---- Themes & settings -------------------------------------------------------

#[tauri::command]
fn list_themes(state: State<'_, AppState>) -> Vec<themes::Theme> {
    themes::list(&state.themes_dir)
}

#[tauri::command]
fn open_themes_dir(state: State<'_, AppState>) -> CmdResult<()> {
    themes::init_dir(&state.themes_dir);
    platform::open_uri(&state.themes_dir.to_string_lossy()).map_err(err_chain)
}

#[tauri::command]
fn open_url(url: String) -> CmdResult<()> {
    // Only web links; store URIs go through their own commands.
    if !url.starts_with("https://") {
        return Err("Only https links can be opened.".into());
    }
    platform::open_uri(&url).map_err(err_chain)
}

#[tauri::command]
fn get_setting(state: State<'_, AppState>, key: String) -> CmdResult<Option<String>> {
    state.db.get_setting(&key).map_err(err)
}

#[tauri::command]
fn set_setting(state: State<'_, AppState>, key: String, value: String) -> CmdResult<()> {
    state.db.set_setting(&key, &value).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let art_dir = data_dir.join("covers");
            std::fs::create_dir_all(&art_dir)?;
            let themes_dir = app.path().app_config_dir()?.join("themes");
            themes::init_dir(&themes_dir);
            let db = Db::open(&data_dir.join("cabinet.db"))?;
            app.manage(AppState {
                db: Arc::new(db),
                sources: Arc::new(sources::all()),
                accounts: Accounts::default(),
                tracker: Tracker::default(),
                art_dir,
                themes_dir,
                art_worker_busy: Arc::new(AtomicBool::new(false)),
                owned_sync_busy: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library,
            get_sources,
            rescan,
            launch_game,
            game_details,
            get_sessions,
            open_install_dir,
            open_store_page,
            get_accounts,
            set_steam_api_key,
            epic_login_url,
            epic_login,
            epic_logout,
            sync_accounts,
            list_themes,
            open_themes_dir,
            open_url,
            get_setting,
            set_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cabinet");
}
