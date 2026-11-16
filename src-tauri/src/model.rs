use std::path::PathBuf;

use serde::Serialize;

/// A game as reported by a store scanner. This is the store-agnostic shape every
/// `GameSource` produces; anything store-specific lives in `meta`.
#[derive(Debug, Clone)]
pub struct Game {
    /// Key of the source that owns this game (`"steam"`, `"epic"`, ...).
    pub source: String,
    /// The store's own identifier (Steam app id, Epic AppName, ...).
    pub source_id: String,
    pub title: String,
    pub install_dir: PathBuf,
    /// Whatever the source needs to start the game, usually a store URI.
    pub launch_target: String,
    /// Free-form, source-specific data persisted alongside the game.
    pub meta: serde_json::Value,
    /// Playtime the store itself has recorded, when it exposes it locally.
    pub store_playtime: Option<StorePlaytime>,
}

/// A game the signed-in store account owns, installed or not.
#[derive(Debug, Clone)]
pub struct OwnedGame {
    pub source: String,
    pub source_id: String,
    pub title: String,
    /// Source-specific data, merged into what the installed scan stores.
    pub meta: serde_json::Value,
    pub store_playtime: Option<StorePlaytime>,
}

impl OwnedGame {
    pub fn id(&self) -> String {
        make_id(&self.source, &self.source_id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StorePlaytime {
    pub secs: i64,
    /// Playtime in roughly the last two weeks, as the store reports it.
    pub recent_secs: i64,
    pub last_played: Option<i64>,
}

impl Game {
    pub fn id(&self) -> String {
        make_id(&self.source, &self.source_id)
    }
}

pub fn make_id(source: &str, source_id: &str) -> String {
    format!("{source}:{source_id}")
}

/// What the UI renders for one tile in the library grid.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGame {
    pub id: String,
    pub source: String,
    pub title: String,
    /// Absolute paths to locally cached artwork, where it exists.
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
    pub added_at: i64,
    pub last_played: Option<i64>,
    pub launch_count: i64,
    /// Total playtime: the store's figure plus Cabinet sessions it hasn't seen yet.
    pub playtime_secs: i64,
    /// The part of `playtime_secs` imported from the store.
    pub store_playtime_secs: i64,
    /// Playtime over the last 14 days (store figure plus newer Cabinet sessions).
    pub recent_secs: i64,
    /// Sessions tracked by Cabinet.
    pub session_count: i64,
    /// False for games that are owned but not installed.
    pub installed: bool,
    pub running: bool,
}

/// One tracked play session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub game_id: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_secs: i64,
}
