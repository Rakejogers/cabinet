//! Background work: fetching artwork and syncing owned libraries from store
//! accounts. Both report progress to the UI through events.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;
use crate::covers::{self, ArtKind};
use crate::db::now;
use crate::sources::SyncContext;

pub const ART_EVENT: &str = "art-updated";
/// Tells the UI to reload the library (after an owned-library sync).
pub const LIBRARY_EVENT: &str = "library-changed";
const ART_WORKERS: usize = 4;

#[derive(Clone, Serialize)]
struct ArtUpdate {
    id: String,
    kind: ArtKind,
    path: String,
}

/// Fetches and caches missing artwork (covers first, since the grid needs
/// them), emitting an event per image so the UI fills in progressively.
pub fn spawn_art_worker(app: &AppHandle, retry_failed: bool) {
    let state = app.state::<AppState>();
    if state.art_worker_busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let (db, sources, dir, busy) =
        (state.db.clone(), state.sources.clone(), state.art_dir.clone(), state.art_worker_busy.clone());
    let app = app.clone();
    thread::spawn(move || {
        for kind in ArtKind::ALL {
            // A few rounds pick up games added by a sync that ran meanwhile.
            for round in 0..3 {
                let todo = db.games_needing_art(kind, retry_failed && round == 0).unwrap_or_default();
                if todo.is_empty() {
                    break;
                }
                let started = Instant::now();
                let found = AtomicUsize::new(0);
                let chunk_size = todo.len().div_ceil(ART_WORKERS);
                thread::scope(|scope| {
                    for chunk in todo.chunks(chunk_size) {
                        let (db, sources, dir, app, found) = (&db, &sources, &dir, &app, &found);
                        scope.spawn(move || {
                            for game in chunk {
                                let id = game.id();
                                let stored = sources
                                    .iter()
                                    .find(|s| s.key() == game.source)
                                    .and_then(|s| s.fetch_art(game, kind))
                                    .and_then(|img| covers::store(dir, &id, kind, &img).ok());
                                let _ = db.set_art(&id, kind, stored.as_deref());
                                if let Some(path) = stored {
                                    found.fetch_add(1, Ordering::Relaxed);
                                    let path = path.to_string_lossy().to_string();
                                    let _ = app.emit(ART_EVENT, ArtUpdate { id, kind, path });
                                }
                            }
                        });
                    }
                });
                eprintln!("[art] {kind:?}: {}/{} found in {:?}", found.into_inner(), todo.len(), started.elapsed());
            }
        }
        busy.store(false, Ordering::SeqCst);
    });
}

/// The outcome of the last owned-library sync for one store, persisted in
/// settings as `sync.<source>`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    pub at: i64,
    pub count: Option<usize>,
    pub error: Option<String>,
}

pub fn last_sync(app: &AppHandle, source: &str) -> Option<SyncInfo> {
    let raw = app.state::<AppState>().db.get_setting(&format!("sync.{source}")).ok()??;
    serde_json::from_str(&raw).ok()
}

/// Syncs owned libraries for every connected store (or just `only`). Runs on
/// the calling thread; callers put it on a background thread.
pub fn sync_owned(app: &AppHandle, only: Option<&str>) {
    let state = app.state::<AppState>();
    if state.owned_sync_busy.swap(true, Ordering::SeqCst) {
        return;
    }
    for source in state.sources.iter().filter(|s| only.is_none_or(|k| k == s.key())) {
        let known = state.db.known_games(source.key()).unwrap_or_default();
        let ctx = SyncContext { accounts: &state.accounts, known: &known };
        let started = Instant::now();
        let info = match source.owned(&ctx) {
            Ok(None) => continue,
            Ok(Some(games)) => match state.db.sync_owned(source.key(), &games) {
                Ok(()) => SyncInfo { at: now(), count: Some(games.len()), error: None },
                Err(e) => SyncInfo { at: now(), count: None, error: Some(format!("{e:#}")) },
            },
            Err(e) => SyncInfo { at: now(), count: None, error: Some(format!("{e:#}")) },
        };
        match &info.error {
            Some(e) => eprintln!("[owned] {}: {e}", source.key()),
            None => eprintln!("[owned] {}: {} games in {:?}", source.key(), info.count.unwrap_or(0), started.elapsed()),
        }
        if let Ok(json) = serde_json::to_string(&info) {
            let _ = state.db.set_setting(&format!("sync.{}", source.key()), &json);
        }
    }
    state.owned_sync_busy.store(false, Ordering::SeqCst);
    let _ = app.emit(LIBRARY_EVENT, ());
    spawn_art_worker(app, false);
}

pub fn spawn_owned_sync(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || sync_owned(&app, None));
}
