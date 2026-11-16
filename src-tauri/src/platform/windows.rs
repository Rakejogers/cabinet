use std::path::{Path, PathBuf};

use super::UbisoftInstall;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

fn reg_string(root: RegKey, key: &str, value: &str) -> Option<String> {
    root.open_subkey(key).ok()?.get_value::<String, _>(value).ok()
}

pub fn steam_root() -> Option<PathBuf> {
    let candidates = [
        reg_string(RegKey::predef(HKEY_CURRENT_USER), r"Software\Valve\Steam", "SteamPath"),
        reg_string(RegKey::predef(HKEY_LOCAL_MACHINE), r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        Some(r"C:\Program Files (x86)\Steam".to_string()),
    ];
    candidates.into_iter().flatten().map(|p| PathBuf::from(p.replace('/', "\\"))).find(|p| p.join("steamapps").is_dir())
}

pub fn epic_manifests_dir() -> Option<PathBuf> {
    let program_data =
        std::env::var_os("PROGRAMDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let dir = program_data.join(r"Epic\EpicGamesLauncher\Data\Manifests");
    dir.is_dir().then_some(dir)
}

/// Windows paths are case-insensitive and accept either slash.
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

const UNINSTALL_KEY: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

pub fn ubisoft_installs() -> Vec<UbisoftInstall> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(installs) = hklm.open_subkey(r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs") else {
        return Vec::new();
    };
    installs
        .enum_keys()
        .flatten()
        .filter_map(|id| {
            let dir: String = installs.open_subkey(&id).ok()?.get_value("InstallDir").ok()?;
            let dir = PathBuf::from(dir.replace('/', "\\"));
            if !dir.is_dir() {
                return None;
            }
            let uninstall = format!(r"{UNINSTALL_KEY}\Uplay Install {id}");
            let name = reg_string(RegKey::predef(HKEY_LOCAL_MACHINE), &uninstall, "DisplayName");
            Some(UbisoftInstall { id, name, dir })
        })
        .collect()
}

/// Folders the Xbox app installs games into. Each drive with Xbox games has a
/// `.GamingRoot` file listing them ("RGBX", a count, then UTF-16 paths).
pub fn xbox_library_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for letter in b'C'..=b'Z' {
        let drive = PathBuf::from(format!(r"{}:\", letter as char));
        let Ok(bytes) = std::fs::read(drive.join(".GamingRoot")) else {
            continue;
        };
        if bytes.len() < 8 || &bytes[..4] != b"RGBX" {
            continue;
        }
        let units: Vec<u16> = bytes[8..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        for path in units.split(|u| *u == 0).filter(|p| !p.is_empty()) {
            let dir = drive.join(String::from_utf16_lossy(path).trim_start_matches('\\'));
            if dir.is_dir() {
                roots.push(dir);
            }
        }
    }
    roots
}
