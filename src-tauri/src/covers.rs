//! Game artwork: image validation and the local art cache.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub struct ArtImage {
    pub bytes: Vec<u8>,
    pub ext: &'static str,
}

/// The kinds of artwork Cabinet caches per game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtKind {
    /// Portrait 2:3 capsule for the grid.
    Cover,
    /// Wide banner for the home screen and detail page.
    Hero,
    /// Transparent title logo drawn over the hero.
    Logo,
}

impl ArtKind {
    pub const ALL: [ArtKind; 3] = [ArtKind::Cover, ArtKind::Hero, ArtKind::Logo];

    fn file_suffix(self) -> &'static str {
        match self {
            ArtKind::Cover => "",
            ArtKind::Hero => "-hero",
            ArtKind::Logo => "-logo",
        }
    }
}

/// Identifies an image by its magic bytes so error pages never get cached.
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        _ => None,
    }
}

pub fn from_bytes(bytes: Vec<u8>) -> Option<ArtImage> {
    let ext = sniff(&bytes)?;
    Some(ArtImage { bytes, ext })
}

pub fn from_file(path: &Path) -> Option<ArtImage> {
    from_bytes(fs::read(path).ok()?)
}

/// Writes artwork into the cache as `<source>-<id>[-kind].<ext>` and removes
/// any stale copy with a different extension.
pub fn store(dir: &Path, game_id: &str, kind: ArtKind, image: &ArtImage) -> Result<PathBuf> {
    let mut stem: String =
        game_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    stem.push_str(kind.file_suffix());
    for ext in ["jpg", "png", "webp"] {
        if ext != image.ext {
            let _ = fs::remove_file(dir.join(format!("{stem}.{ext}")));
        }
    }
    let path = dir.join(format!("{stem}.{}", image.ext));
    let tmp = dir.join(format!("{stem}.tmp"));
    fs::write(&tmp, &image.bytes)?;
    fs::rename(&tmp, &path)?;
    Ok(path)
}
