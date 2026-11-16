//! Fallback for platforms Cabinet doesn't target yet (Linux, BSD), so the
//! crate still compiles there.

use std::path::{Path, PathBuf};

use super::UbisoftInstall;

pub fn steam_root() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("HOME")?).join(".local/share/Steam");
    dir.join("steamapps").is_dir().then_some(dir)
}

pub fn epic_manifests_dir() -> Option<PathBuf> {
    None
}

pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

/// Ubisoft Connect is Windows-only.
pub fn ubisoft_installs() -> Vec<UbisoftInstall> {
    Vec::new()
}

/// The Xbox app is Windows-only.
pub fn xbox_library_roots() -> Vec<PathBuf> {
    Vec::new()
}
