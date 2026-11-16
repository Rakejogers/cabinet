//! Everything that differs between operating systems lives under this module.
//! Each OS file exposes the same functions; the rest of the app never uses
//! `#[cfg(target_os)]` directly.

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod other;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub use other::*;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// A game registered by Ubisoft Connect.
pub struct UbisoftInstall {
    pub id: String,
    pub name: Option<String>,
    pub dir: PathBuf,
}

/// Hands a URI (`steam://…`, `com.epicgames.launcher://…`) or path to the OS.
pub fn open_uri(uri: &str) -> Result<()> {
    open::that_detached(uri).with_context(|| format!("could not open {uri}"))
}

/// True when `child` is `parent` or lies somewhere beneath it, using the
/// platform's path comparison rules.
pub fn path_is_within(child: &Path, parent: &Path) -> bool {
    let child = normalize_path(child);
    let parent = normalize_path(parent);
    if parent.is_empty() {
        return false;
    }
    child == parent || (child.starts_with(&parent) && child[parent.len()..].starts_with(std::path::MAIN_SEPARATOR))
}
