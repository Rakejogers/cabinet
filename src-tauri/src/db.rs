//! SQLite persistence for the library, play sessions and settings.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::covers::ArtKind;
use crate::model::{Game, LibraryGame, OwnedGame, Session};

/// Schema migrations, applied in order and tracked with `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    r#"
CREATE TABLE games (
    id               TEXT PRIMARY KEY,
    source           TEXT NOT NULL,
    source_id        TEXT NOT NULL,
    title            TEXT NOT NULL,
    install_dir      TEXT NOT NULL,
    launch_target    TEXT NOT NULL,
    meta             TEXT NOT NULL DEFAULT '{}',
    installed        INTEGER NOT NULL DEFAULT 1,
    cover_path       TEXT,
    cover_checked_at INTEGER,
    added_at         INTEGER NOT NULL,
    last_played      INTEGER,
    launch_count     INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE sessions (
    id            INTEGER PRIMARY KEY,
    game_id       TEXT NOT NULL REFERENCES games(id),
    started_at    INTEGER NOT NULL,
    ended_at      INTEGER NOT NULL,
    duration_secs INTEGER NOT NULL
);
CREATE INDEX sessions_game ON sessions(game_id);
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#,
    r#"
-- Playtime imported from the store. `store_synced_at` is when the imported
-- figure last *changed*; Cabinet sessions ending after it are not in it yet.
ALTER TABLE games ADD COLUMN store_playtime_secs INTEGER;
ALTER TABLE games ADD COLUMN store_last_played INTEGER;
ALTER TABLE games ADD COLUMN store_synced_at INTEGER;
"#,
    r#"
ALTER TABLE games ADD COLUMN hero_path TEXT;
ALTER TABLE games ADD COLUMN hero_checked_at INTEGER;
ALTER TABLE games ADD COLUMN logo_path TEXT;
ALTER TABLE games ADD COLUMN logo_checked_at INTEGER;
ALTER TABLE games ADD COLUMN store_recent_secs INTEGER;
"#,
    r#"
-- Owned through a signed-in store account (installed or not).
ALTER TABLE games ADD COLUMN owned INTEGER NOT NULL DEFAULT 0;
CREATE INDEX sessions_started ON sessions(started_at);
"#,
];

/// How a fresh store figure updates the imported playtime columns. Playtime
/// only grows, so the larger figure wins (local files and the web API can
/// disagree slightly), and `store_synced_at` moves only when it grew.
const STORE_PLAYTIME_UPDATE: &str = "
    store_synced_at = CASE WHEN excluded.store_playtime_secs > COALESCE(games.store_playtime_secs, -1)
        THEN excluded.store_synced_at ELSE games.store_synced_at END,
    store_playtime_secs = CASE WHEN excluded.store_playtime_secs > COALESCE(games.store_playtime_secs, -1)
        THEN excluded.store_playtime_secs ELSE games.store_playtime_secs END,
    store_last_played = NULLIF(MAX(COALESCE(excluded.store_last_played, 0), COALESCE(games.store_last_played, 0)), 0),
    store_recent_secs = COALESCE(excluded.store_recent_secs, games.store_recent_secs)";

/// Columns of `LibraryGame`, in order, for the game aliased `g`.
fn library_columns() -> String {
    format!(
        "g.id, g.source, g.title, g.cover_path, g.added_at,
         NULLIF(MAX(COALESCE(g.last_played, 0), COALESCE(g.store_last_played, 0)), 0),
         g.launch_count, {TOTAL_PLAYTIME_SQL},
         (SELECT COUNT(*) FROM sessions s WHERE s.game_id = g.id),
         COALESCE(g.store_playtime_secs, 0), {RECENT_PLAYTIME_SQL},
         g.hero_path, g.logo_path, g.installed"
    )
}

fn library_row(r: &rusqlite::Row) -> rusqlite::Result<LibraryGame> {
    Ok(LibraryGame {
        id: r.get(0)?,
        source: r.get(1)?,
        title: r.get(2)?,
        cover: r.get(3)?,
        added_at: r.get(4)?,
        last_played: r.get(5)?,
        launch_count: r.get(6)?,
        playtime_secs: r.get(7)?,
        session_count: r.get(8)?,
        store_playtime_secs: r.get(9)?,
        recent_secs: r.get(10)?,
        hero: r.get(11)?,
        logo: r.get(12)?,
        installed: r.get(13)?,
        running: false,
    })
}

/// Total playtime for the game aliased `g`. Stores like Steam also count the
/// sessions Cabinet launches, so only sessions newer than the store's last
/// update are added on top of its figure.
const TOTAL_PLAYTIME_SQL: &str = "COALESCE(g.store_playtime_secs, 0) + COALESCE((
        SELECT SUM(s.duration_secs) FROM sessions s
        WHERE s.game_id = g.id AND s.ended_at > COALESCE(g.store_synced_at, 0)), 0)";

/// Same rule as `TOTAL_PLAYTIME_SQL`, limited to the last 14 days.
const RECENT_PLAYTIME_SQL: &str = "COALESCE(g.store_recent_secs, 0) + COALESCE((
        SELECT SUM(s.duration_secs) FROM sessions s
        WHERE s.game_id = g.id AND s.ended_at > COALESCE(g.store_synced_at, 0)
          AND s.ended_at > CAST(strftime('%s', 'now') AS INTEGER) - 1209600), 0)";

/// The (path, checked-at) columns that hold one kind of artwork.
fn art_columns(kind: ArtKind) -> (&'static str, &'static str) {
    match kind {
        ArtKind::Cover => ("cover_path", "cover_checked_at"),
        ArtKind::Hero => ("hero_path", "hero_checked_at"),
        ArtKind::Logo => ("logo_path", "logo_checked_at"),
    }
}

/// How long to wait before retrying a game whose cover couldn't be found.
const COVER_RETRY_SECS: i64 = 24 * 60 * 60;

pub struct Db {
    conn: Mutex<Connection>,
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version.max(0) as usize) {
            conn.execute_batch(&format!("BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;", i + 1))?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Replaces what we know about one source's installed games with a fresh
    /// scan. Games that disappeared are kept (with their playtime) but hidden.
    pub fn sync_source(&self, source: &str, games: &[Game]) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let now = now();
        {
            let mut upsert = tx.prepare(&format!(
                "INSERT INTO games (id, source, source_id, title, install_dir, launch_target, meta, installed, added_at,
                                    store_playtime_secs, store_last_played, store_synced_at, store_recent_secs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, CASE WHEN ?9 IS NULL THEN NULL ELSE ?8 END, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title, install_dir = excluded.install_dir,
                    launch_target = excluded.launch_target, installed = 1,
                    meta = json_patch(games.meta, excluded.meta),
                    {STORE_PLAYTIME_UPDATE}"
            ))?;
            for g in games {
                upsert.execute(params![
                    g.id(),
                    g.source,
                    g.source_id,
                    g.title,
                    g.install_dir.to_string_lossy(),
                    g.launch_target,
                    g.meta.to_string(),
                    now,
                    g.store_playtime.map(|p| p.secs),
                    g.store_playtime.and_then(|p| p.last_played),
                    g.store_playtime.map(|p| p.recent_secs),
                ])?;
            }
            let seen: HashSet<String> = games.iter().map(Game::id).collect();
            let mut existing = tx.prepare("SELECT id FROM games WHERE source = ?1 AND installed = 1")?;
            let gone: Vec<String> = existing
                .query_map([source], |r| r.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .filter(|id| !seen.contains(id))
                .collect();
            for id in gone {
                tx.execute("UPDATE games SET installed = 0 WHERE id = ?1", [id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Records what a store account owns. Installed details (title, folder)
    /// from the local scan take precedence; games no longer owned lose the flag.
    pub fn sync_owned(&self, source: &str, games: &[OwnedGame]) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let now = now();
        {
            let mut upsert = tx.prepare(&format!(
                "INSERT INTO games (id, source, source_id, title, install_dir, launch_target, meta, installed, owned,
                                    added_at, store_playtime_secs, store_last_played, store_synced_at, store_recent_secs)
                 VALUES (?1, ?2, ?3, ?4, '', '', ?5, 0, 1, ?6, ?7, ?8, CASE WHEN ?7 IS NULL THEN NULL ELSE ?6 END, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    owned = 1,
                    title = CASE WHEN games.installed = 1 THEN games.title ELSE excluded.title END,
                    meta = json_patch(excluded.meta, games.meta),
                    {STORE_PLAYTIME_UPDATE}"
            ))?;
            for g in games {
                upsert.execute(params![
                    g.id(),
                    g.source,
                    g.source_id,
                    g.title,
                    g.meta.to_string(),
                    now,
                    g.store_playtime.map(|p| p.secs),
                    g.store_playtime.and_then(|p| p.last_played),
                    g.store_playtime.map(|p| p.recent_secs),
                ])?;
            }
            let seen: HashSet<String> = games.iter().map(OwnedGame::id).collect();
            let mut existing = tx.prepare("SELECT id FROM games WHERE source = ?1 AND owned = 1")?;
            let gone: Vec<String> = existing
                .query_map([source], |r| r.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .filter(|id| !seen.contains(id))
                .collect();
            for id in gone {
                tx.execute("UPDATE games SET owned = 0 WHERE id = ?1", [id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Clears the owned flag for a store whose account was disconnected.
    pub fn clear_owned(&self, source: &str) -> Result<()> {
        self.conn().execute("UPDATE games SET owned = 0 WHERE source = ?1", [source])?;
        Ok(())
    }

    /// Installed games plus everything a connected store account owns.
    pub fn library(&self) -> Result<Vec<LibraryGame>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM games g WHERE g.installed = 1 OR g.owned = 1 ORDER BY g.title COLLATE NOCASE",
            library_columns()
        ))?;
        let rows = stmt.query_map([], library_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn library_game(&self, id: &str) -> Result<Option<LibraryGame>> {
        let conn = self.conn();
        Ok(conn
            .query_row(&format!("SELECT {} FROM games g WHERE g.id = ?1", library_columns()), [id], library_row)
            .optional()?)
    }

    /// Games of one source already stored, keyed by store id.
    pub fn known_games(&self, source: &str) -> Result<HashMap<String, Game>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT source, source_id, title, install_dir, launch_target, meta FROM games WHERE source = ?1",
        )?;
        let rows = stmt.query_map([source], Self::game_from_row)?;
        Ok(rows.filter_map(|r| r.ok()).map(|g| (g.source_id.clone(), g)).collect())
    }

    /// Sessions that ended after `since`, optionally for a single game.
    pub fn sessions(&self, game_id: Option<&str>, since: i64) -> Result<Vec<Session>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT game_id, started_at, ended_at, duration_secs FROM sessions
             WHERE ended_at >= ?1 AND (?2 IS NULL OR game_id = ?2) AND duration_secs > 0
             ORDER BY started_at",
        )?;
        let rows = stmt.query_map(params![since, game_id], |r| {
            Ok(Session { game_id: r.get(0)?, started_at: r.get(1)?, ended_at: r.get(2)?, duration_secs: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn game_from_row(r: &rusqlite::Row) -> rusqlite::Result<Game> {
        let meta: String = r.get(5)?;
        Ok(Game {
            source: r.get(0)?,
            source_id: r.get(1)?,
            title: r.get(2)?,
            install_dir: PathBuf::from(r.get::<_, String>(3)?),
            launch_target: r.get(4)?,
            meta: serde_json::from_str(&meta).unwrap_or_default(),
            store_playtime: None,
        })
    }

    /// A stored game and whether it is installed.
    pub fn game(&self, id: &str) -> Result<Option<(Game, bool)>> {
        let conn = self.conn();
        Ok(conn
            .query_row(
                "SELECT source, source_id, title, install_dir, launch_target, meta, installed FROM games WHERE id = ?1",
                [id],
                |r| Ok((Self::game_from_row(r)?, r.get::<_, bool>(6)?)),
            )
            .optional()?)
    }

    /// Installed games whose artwork of `kind` is missing on disk and is due
    /// for a lookup (`retry_failed` skips the back-off for earlier misses).
    pub fn games_needing_art(&self, kind: ArtKind, retry_failed: bool) -> Result<Vec<Game>> {
        let conn = self.conn();
        let (path_col, checked_col) = art_columns(kind);
        let mut stmt = conn.prepare(&format!(
            "SELECT source, source_id, title, install_dir, launch_target, meta, {path_col}, {checked_col}
             FROM games WHERE installed = 1 OR owned = 1"
        ))?;
        let cutoff = if retry_failed { i64::MAX } else { now() - COVER_RETRY_SECS };
        let rows = stmt.query_map([], |r| {
            let game = Self::game_from_row(r)?;
            let cover: Option<String> = r.get(6)?;
            let checked: Option<i64> = r.get(7)?;
            Ok((game, cover, checked))
        })?;
        Ok(rows
            .filter_map(|r| r.ok())
            .filter(|(_, cover, checked)| match cover {
                Some(path) => !Path::new(path).is_file(),
                None => checked.is_none_or(|t| t < cutoff),
            })
            .map(|(game, _, _)| game)
            .collect())
    }

    pub fn set_art(&self, game_id: &str, kind: ArtKind, path: Option<&Path>) -> Result<()> {
        let (path_col, checked_col) = art_columns(kind);
        self.conn().execute(
            &format!("UPDATE games SET {path_col} = ?2, {checked_col} = ?3 WHERE id = ?1"),
            params![game_id, path.map(|p| p.to_string_lossy().to_string()), now()],
        )?;
        Ok(())
    }

    pub fn record_launch(&self, game_id: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE games SET launch_count = launch_count + 1, last_played = ?2 WHERE id = ?1",
            params![game_id, now()],
        )?;
        Ok(())
    }

    pub fn start_session(&self, game_id: &str, started_at: i64) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO sessions (game_id, started_at, ended_at, duration_secs) VALUES (?1, ?2, ?2, 0)",
            params![game_id, started_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Extends a running session; called periodically so a crash loses little.
    pub fn update_session(&self, session_id: i64, ended_at: i64) -> Result<()> {
        self.conn().execute(
            "UPDATE sessions SET ended_at = ?2, duration_secs = MAX(0, ?2 - started_at) WHERE id = ?1",
            params![session_id, ended_at],
        )?;
        Ok(())
    }

    pub fn finish_session(&self, game_id: &str, session_id: i64, ended_at: i64) -> Result<()> {
        self.update_session(session_id, ended_at)?;
        self.conn().execute("UPDATE games SET last_played = ?2 WHERE id = ?1", params![game_id, ended_at])?;
        Ok(())
    }

    pub fn playtime(&self, game_id: &str) -> Result<(i64, i64)> {
        Ok(self.conn().query_row(
            &format!(
                "SELECT {TOTAL_PLAYTIME_SQL}, (SELECT COUNT(*) FROM sessions s WHERE s.game_id = g.id)
                 FROM games g WHERE g.id = ?1"
            ),
            [game_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn().query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StorePlaytime;

    fn steam_game(secs: Option<i64>) -> Game {
        Game {
            source: "steam".into(),
            source_id: "42".into(),
            title: "Test".into(),
            install_dir: PathBuf::from("C:/games/test"),
            launch_target: "steam://rungameid/42".into(),
            meta: serde_json::json!({}),
            store_playtime: secs.map(|secs| StorePlaytime { secs, recent_secs: 0, last_played: Some(1_000) }),
        }
    }

    #[test]
    fn store_playtime_is_not_double_counted() {
        let path = std::env::temp_dir().join(format!("cabinet-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        let id = "steam:42";

        db.sync_source("steam", &[steam_game(Some(600))]).unwrap();
        assert_eq!(db.playtime(id).unwrap(), (600, 0));

        // Pretend Steam's figure was last updated a while ago, then play 60s via Cabinet.
        let now = now();
        db.conn().execute("UPDATE games SET store_synced_at = ?1", [now - 1000]).unwrap();
        let session = db.start_session(id, now - 200).unwrap();
        db.finish_session(id, session, now - 140).unwrap();
        assert_eq!(db.playtime(id).unwrap(), (660, 1), "session not yet in Steam's figure");

        // Steam hasn't written its file yet: same figure, session still added on top.
        db.sync_source("steam", &[steam_game(Some(600))]).unwrap();
        assert_eq!(db.playtime(id).unwrap(), (660, 1));

        // Steam catches up and now includes the session itself.
        db.sync_source("steam", &[steam_game(Some(660))]).unwrap();
        assert_eq!(db.playtime(id).unwrap(), (660, 1));

        // A scan without store data keeps the imported figure.
        db.sync_source("steam", &[steam_game(None)]).unwrap();
        assert_eq!(db.playtime(id).unwrap(), (660, 1));

        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
