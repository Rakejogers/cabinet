use std::path::{Path, PathBuf};

use super::UbisoftInstall;

fn app_support() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
}

pub fn steam_root() -> Option<PathBuf> {
    let dir = app_support()?.join("Steam");
    dir.join("steamapps").is_dir().then_some(dir)
}

pub fn epic_manifests_dir() -> Option<PathBuf> {
    let dir = app_support()?.join("Epic/EpicGamesLauncher/Data/Manifests");
    dir.is_dir().then_some(dir)
}

/// APFS volumes are case-insensitive by default.
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_lowercase()
}

/// Ubisoft Connect is Windows-only.
pub fn ubisoft_installs() -> Vec<UbisoftInstall> {
    Vec::new()
}

/// The Xbox app is Windows-only.
pub fn xbox_library_roots() -> Vec<PathBuf> {
    Vec::new()
}
