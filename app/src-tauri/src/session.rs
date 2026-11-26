//! The signed-in session: the access token in memory, the refresh token in
//! the OS keychain (Windows Credential Manager, macOS/iOS Keychain, Linux
//! Secret Service), so a restart signs straight back in.

use crate::supabase::{Session, Supabase};
use lattice_core::api::{ApiError, AuthUser};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::Mutex;

/// Refresh this long before the access token expires.
const REFRESH_MARGIN_SECS: i64 = 60;

/// What survives a restart. Kept small: Windows caps a credential at 2.5 KB.
#[derive(Serialize, Deserialize)]
struct Stored {
    refresh_token: String,
    user: AuthUser,
}

#[cfg(any(windows, target_os = "macos", target_os = "ios", target_os = "linux"))]
mod keychain {
    fn entry() -> Option<keyring::Entry> {
        keyring::Entry::new("com.latticelane.hamperbuilder", "session").ok()
    }

    /// `Err` when the keychain can't be used, so the caller falls back to a file.
    pub fn read() -> Result<Option<String>, ()> {
        match entry().ok_or(())?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => {
                tracing::warn!("keychain read failed: {e}");
                Err(())
            }
        }
    }

    pub fn write(value: &str) -> bool {
        entry().is_some_and(|e| e.set_password(value).map_err(|e| tracing::warn!("keychain write failed: {e}")).is_ok())
    }

    pub fn delete() {
        if let Some(e) = entry() {
            let _ = e.delete_credential();
        }
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "linux")))]
mod keychain {
    // ponytail: no keyring backend on Android yet, so the file fallback is used; Phase 6 moves it to the Keystore.
    pub fn read() -> Result<Option<String>, ()> {
        Err(())
    }
    pub fn write(_: &str) -> bool {
        false
    }
    pub fn delete() {}
}

/// The keychain where the platform has one, else a file in the app's data folder.
pub struct SecretStore {
    fallback: PathBuf,
}

impl SecretStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { fallback: data_dir.join("session.json") }
    }

    fn read(&self) -> Option<String> {
        match keychain::read() {
            Ok(Some(s)) => Some(s),
            _ => std::fs::read_to_string(&self.fallback).ok(),
        }
    }

    fn write(&self, value: &str) {
        if keychain::write(value) {
            let _ = std::fs::remove_file(&self.fallback);
        } else if let Err(e) = std::fs::write(&self.fallback, value) {
            tracing::error!("could not save the session: {e}");
        }
    }

    fn delete(&self) {
        keychain::delete();
        let _ = std::fs::remove_file(&self.fallback);
    }
}

pub struct Sessions {
    current: Mutex<Option<Session>>,
    store: SecretStore,
}

impl Sessions {
    /// The session saved last time, needing a refresh before its first use.
    pub fn load(store: SecretStore) -> Self {
        let saved = store.read().and_then(|s| serde_json::from_str::<Stored>(&s).ok()).map(|s| Session {
            access_token: String::new(),
            refresh_token: s.refresh_token,
            expires_at: 0,
            user: s.user,
        });
        Self { current: Mutex::new(saved), store }
    }

    pub async fn user(&self) -> Option<AuthUser> {
        self.current.lock().await.as_ref().map(|s| s.user.clone())
    }

    fn persist(&self, s: &Session) {
        let stored = Stored { refresh_token: s.refresh_token.clone(), user: s.user.clone() };
        if let Ok(json) = serde_json::to_string(&stored) {
            self.store.write(&json);
        }
    }

    pub async fn set(&self, s: Session) {
        self.persist(&s);
        *self.current.lock().await = Some(s);
    }

    pub async fn clear(&self) -> Option<Session> {
        self.store.delete();
        self.current.lock().await.take()
    }

    /// A valid access token, refreshing it first when it is about to expire.
    /// The lock is held across the refresh, so concurrent calls refresh once.
    pub async fn token(&self, api: &Supabase) -> Result<String, ApiError> {
        let mut guard = self.current.lock().await;
        let Some(session) = guard.as_ref() else { return Err(ApiError::SignedOut) };
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if !session.access_token.is_empty() && session.expires_at - now > REFRESH_MARGIN_SECS {
            return Ok(session.access_token.clone());
        }
        match api.refresh(&session.refresh_token).await {
            Ok(fresh) => {
                // Refresh tokens rotate: save the new one before anything else can fail.
                self.persist(&fresh);
                let token = fresh.access_token.clone();
                *guard = Some(fresh);
                Ok(token)
            }
            Err(ApiError::SignedOut) => {
                *guard = None;
                self.store.delete();
                Err(ApiError::SignedOut)
            }
            // Offline and the like: keep the session for when the network is back.
            Err(other) => Err(other),
        }
    }
}
