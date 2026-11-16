//! Ubisoft Connect: installs are listed in the registry (via `platform`),
//! launches go through the `uplay://` URI handler. Ubisoft has no public art
//! service, so artwork is borrowed from the same game's Steam listing.

use anyhow::Result;

use super::{GameSource, art_from_steam_by_title};
use crate::covers::{ArtImage, ArtKind};
use crate::model::Game;
use crate::platform;

pub struct UbisoftSource;

impl GameSource for UbisoftSource {
    fn key(&self) -> &'static str {
        "ubisoft"
    }

    fn name(&self) -> &'static str {
        "Ubisoft Connect"
    }

    fn scan(&self) -> Vec<Game> {
        platform::ubisoft_installs()
            .into_iter()
            .map(|install| {
                let folder = install.dir.file_name().map(|n| n.to_string_lossy().to_string());
                let title = install.name.or(folder).unwrap_or_else(|| format!("Ubisoft game {}", install.id));
                Game {
                    source: "ubisoft".into(),
                    source_id: install.id,
                    title,
                    install_dir: install.dir,
                    launch_target: String::new(),
                    meta: serde_json::json!({}),
                    store_playtime: None,
                }
            })
            .collect()
    }

    fn launch(&self, game: &Game) -> Result<()> {
        platform::open_uri(&format!("uplay://launch/{}/0", game.source_id))
    }

    fn fetch_art(&self, game: &Game, kind: ArtKind) -> Option<ArtImage> {
        art_from_steam_by_title(&game.title, kind)
    }
}
