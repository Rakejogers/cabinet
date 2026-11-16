//! Store integrations. Each store implements `GameSource`; OS differences
//! (install locations, path rules, registry) are delegated to `crate::platform`.

mod epic;
pub mod epic_api;
mod steam;
mod ubisoft;
pub mod vdf;
mod xbox;

use std::collections::HashMap;

use anyhow::Result;

use crate::accounts::Accounts;
use crate::covers::{ArtImage, ArtKind};
use crate::model::{Game, OwnedGame};

/// What an owned-library sync can use besides the network.
pub struct SyncContext<'a> {
    pub accounts: &'a Accounts,
    /// Games of this source already in the database, keyed by store id, so
    /// expensive metadata lookups can be skipped for them.
    pub known: &'a HashMap<String, Game>,
}

pub trait GameSource: Send + Sync {
    /// Stable key stored in the database (`"steam"`, `"epic"`).
    fn key(&self) -> &'static str;

    /// Display name of the store.
    fn name(&self) -> &'static str;

    /// Returns every installed game this store knows about. A store that isn't
    /// installed simply yields an empty list.
    fn scan(&self) -> Vec<Game>;

    /// Starts the game through the store.
    fn launch(&self, game: &Game) -> Result<()>;

    /// Asks the store to install a game that is owned but not installed.
    fn install(&self, game: &Game) -> Result<()> {
        self.launch(game)
    }

    /// A link to the game's store page, if the store has one we can build.
    fn store_page(&self, _game: &Game) -> Option<String> {
        None
    }

    /// Every game the signed-in account owns, installed or not.
    /// `Ok(None)` means no account is configured for this store.
    fn owned(&self, _ctx: &SyncContext) -> Result<Option<Vec<OwnedGame>>> {
        Ok(None)
    }

    /// Finds artwork of the given kind, local files first, network last.
    fn fetch_art(&self, _game: &Game, _kind: ArtKind) -> Option<ArtImage> {
        None
    }
}

pub fn all() -> Vec<Box<dyn GameSource>> {
    vec![
        Box::new(steam::SteamSource),
        Box::new(epic::EpicSource),
        Box::new(ubisoft::UbisoftSource),
        Box::new(xbox::XboxSource),
    ]
}

/// Artwork for games from stores without their own art service: looks the
/// title up on Steam and borrows that game's artwork when the names match.
pub fn art_from_steam_by_title(title: &str, kind: ArtKind) -> Option<ArtImage> {
    let app_id = steam::find_app_by_title(title)?;
    steam::fetch_art_by_id(&app_id.to_string(), kind)
}

/// Display name of the signed-in Steam account, if Steam is installed.
pub fn steam_account_name() -> Option<String> {
    steam::account_name()
}
