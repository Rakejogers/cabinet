//! Playtime tracking. Stores launch games indirectly (through Steam or the Epic
//! launcher), so instead of owning the child process we watch for any process
//! whose executable lives inside the game's install folder.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Emitter};

use crate::db::{Db, now};
use crate::platform;

/// How long a launch may take before we give up looking for the game process
/// (covers store start-up, updates and launchers/splash screens).
const DETECT_TIMEOUT: Duration = Duration::from_secs(180);
const DETECT_INTERVAL: Duration = Duration::from_secs(2);
const RUNNING_INTERVAL: Duration = Duration::from_secs(5);
/// Consecutive empty polls before a session counts as ended. Bridges the gap
/// when a bootstrapper hands over to the real game executable.
const EXIT_GRACE_POLLS: u32 = 3;

pub const STATUS_EVENT: &str = "game-status";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub id: String,
    /// `launching`, `running`, `stopped` or `not-detected`.
    pub state: &'static str,
    pub playtime_secs: i64,
    pub session_count: i64,
    pub last_played: Option<i64>,
}

#[derive(Clone, Default)]
pub struct Tracker {
    active: Arc<Mutex<HashSet<String>>>,
}

impl Tracker {
    pub fn running_ids(&self) -> HashSet<String> {
        self.active.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Starts watching for the game; returns false if it is already watched.
    pub fn track(&self, app: AppHandle, db: Arc<Db>, game_id: String, install_dir: PathBuf) -> bool {
        if !self.active.lock().is_ok_and(|mut s| s.insert(game_id.clone())) {
            return false;
        }
        let active = self.active.clone();
        thread::spawn(move || {
            watch(&app, &db, &game_id, &install_dir);
            if let Ok(mut s) = active.lock() {
                s.remove(&game_id);
            }
        });
        true
    }
}

fn emit(app: &AppHandle, db: &Db, id: &str, state: &'static str, last_played: Option<i64>) {
    let (playtime_secs, session_count) = db.playtime(id).unwrap_or((0, 0));
    let _ = app.emit(STATUS_EVENT, GameStatus { id: id.to_string(), state, playtime_secs, session_count, last_played });
}

fn is_game_process(exe: &Path, install_dir: &Path) -> bool {
    if !platform::path_is_within(exe, install_dir) {
        return false;
    }
    // Crash reporters linger after the game exits and would inflate playtime.
    let name = exe.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    !(name.contains("crashhandler") || name.contains("crashreport"))
}

fn game_running(sys: &mut System, install_dir: &Path) -> bool {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    sys.processes().values().any(|p| p.exe().is_some_and(|exe| is_game_process(exe, install_dir)))
}

fn watch(app: &AppHandle, db: &Db, game_id: &str, install_dir: &Path) {
    // Guard against a bogus install dir (e.g. a drive root) matching everything.
    if install_dir.components().count() < 3 || !install_dir.is_dir() {
        eprintln!("[tracker] {game_id}: unusable install dir {}", install_dir.display());
        emit(app, db, game_id, "not-detected", None);
        return;
    }
    let Ok(mut sys) = System::new() else {
        eprintln!("[tracker] could not initialise process list");
        emit(app, db, game_id, "not-detected", None);
        return;
    };

    emit(app, db, game_id, "launching", None);
    let deadline = Instant::now() + DETECT_TIMEOUT;
    while !game_running(&mut sys, install_dir) {
        if Instant::now() > deadline {
            eprintln!("[tracker] {game_id}: no process found in {}", install_dir.display());
            emit(app, db, game_id, "not-detected", None);
            return;
        }
        thread::sleep(DETECT_INTERVAL);
    }

    let started = now();
    let Ok(session) = db.start_session(game_id, started) else {
        emit(app, db, game_id, "not-detected", None);
        return;
    };
    emit(app, db, game_id, "running", Some(started));

    let mut last_seen = started;
    let mut misses = 0;
    loop {
        thread::sleep(RUNNING_INTERVAL);
        if game_running(&mut sys, install_dir) {
            misses = 0;
            last_seen = now();
            let _ = db.update_session(session, last_seen);
        } else {
            misses += 1;
            if misses >= EXIT_GRACE_POLLS {
                break;
            }
        }
    }
    let _ = db.finish_session(game_id, session, last_seen);
    emit(app, db, game_id, "stopped", Some(last_seen));
}
