//! Store account credentials. Secrets live in the OS credential store
//! (Windows Credential Manager, macOS Keychain), never in the database.

use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::db::now;
use crate::sources::epic_api;

const SERVICE: &str = "com.cabinet.app";
const STEAM_KEY: &str = "steam-api-key";
const EPIC_SESSION: &str = "epic-session";

fn get_secret(name: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, name).ok()?.get_password().ok()
}

fn set_secret(name: &str, value: Option<&str>) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    match value {
        Some(v) => entry.set_password(v)?,
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.into()),
        },
    }
    Ok(())
}

/// What we keep for Epic between runs. The refresh token is the only secret.
#[derive(Serialize, Deserialize, Clone)]
struct EpicStored {
    refresh_token: String,
    account_id: String,
    display_name: String,
}

#[derive(Clone)]
pub struct EpicAccess {
    pub token: String,
    pub account_id: String,
}

#[derive(Default)]
pub struct Accounts {
    /// Short-lived Epic access token, kept in memory only.
    epic_access: Mutex<Option<(EpicAccess, i64)>>,
}

impl Accounts {
    pub fn steam_api_key(&self) -> Option<String> {
        get_secret(STEAM_KEY).filter(|k| !k.is_empty())
    }

    pub fn set_steam_api_key(&self, key: Option<&str>) -> Result<()> {
        set_secret(STEAM_KEY, key.map(str::trim).filter(|k| !k.is_empty()))
    }

    pub fn epic_display_name(&self) -> Option<String> {
        self.epic_stored().map(|s| s.display_name)
    }

    fn epic_stored(&self) -> Option<EpicStored> {
        serde_json::from_str(&get_secret(EPIC_SESSION)?).ok()
    }

    fn save_epic(&self, tokens: &epic_api::Tokens) -> Result<EpicAccess> {
        let stored = EpicStored {
            refresh_token: tokens.refresh_token.clone(),
            account_id: tokens.account_id.clone(),
            display_name: tokens.display_name.clone(),
        };
        set_secret(EPIC_SESSION, Some(&serde_json::to_string(&stored)?))?;
        let access = EpicAccess { token: tokens.access_token.clone(), account_id: tokens.account_id.clone() };
        // Refresh a few minutes before Epic would reject the token.
        let expires = now() + tokens.expires_in.saturating_sub(300);
        *self.epic_access.lock().unwrap_or_else(|e| e.into_inner()) = Some((access.clone(), expires));
        Ok(access)
    }

    /// Exchanges the one-time code from Epic's login page for a session.
    pub fn epic_login(&self, code: &str) -> Result<String> {
        let tokens = epic_api::exchange_code(code)?;
        self.save_epic(&tokens)?;
        Ok(tokens.display_name)
    }

    pub fn epic_logout(&self) -> Result<()> {
        *self.epic_access.lock().unwrap_or_else(|e| e.into_inner()) = None;
        set_secret(EPIC_SESSION, None)
    }

    /// A valid access token, refreshing the session when needed. `Ok(None)`
    /// means the user hasn't signed in.
    pub fn epic_access(&self) -> Result<Option<EpicAccess>> {
        let cached = self.epic_access.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some((access, expires)) = cached
            && now() < expires
        {
            return Ok(Some(access));
        }
        let Some(stored) = self.epic_stored() else {
            return Ok(None);
        };
        let tokens =
            epic_api::refresh(&stored.refresh_token).context("Epic session expired; sign in again in Settings")?;
        Ok(Some(self.save_epic(&tokens)?))
    }
}
