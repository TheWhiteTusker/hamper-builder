//! The contract between the UI (WASM in the webview) and the native app:
//! command arguments, results and errors. Both sides compile this module,
//! so a change here that one side misses fails to build instead of at runtime.

use crate::records::{HamperSummary, Profile, QuoteSummary};
use serde::{Deserialize, Serialize};

/// Every failure the UI can be told about. Messages are written for the
/// person using the app, not for a log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum ApiError {
    /// No connection: cached data is still shown, saves are kept as drafts.
    Offline(String),
    /// The session is gone or expired beyond refresh: sign in again.
    SignedOut,
    /// Row level security (or a role check) refused it.
    Forbidden(String),
    NotFound(String),
    /// A unique code or document number is already taken.
    Conflict(String),
    /// The input was refused: wrong password, missing field, bad value.
    Invalid(String),
    /// Anything else the server or app reported.
    Server(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignedOut => f.write_str("Your session has ended. Please sign in again."),
            Self::Offline(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::Conflict(m)
            | Self::Invalid(m)
            | Self::Server(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
}

/// Who is signed in. `profile` may come from the cache, so it shows instantly.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user: Option<AuthUser>,
    pub profile: Option<Profile>,
    /// Signed in from a password-reset link: the next step is a new password.
    #[serde(default)]
    pub recovering: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SignUpOutcome {
    SignedIn,
    /// Email confirmation is on: the account exists once the link is clicked.
    ConfirmEmail,
}

/// Data the UI asks for by name. Each is cached on the device, so a screen
/// opens with the last copy at once and refreshes in the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    Profile,
    Dashboard,
}

impl Query {
    pub fn key(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Dashboard => "dashboard",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowUp {
    pub doc_no: String,
    pub client_name: String,
    pub follow_up_date: Option<String>,
    pub status: String,
}

/// The dashboard: recent hampers and quotes, active products, follow-ups due.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dashboard {
    pub hampers: Vec<HamperSummary>,
    pub quotes: Vec<QuoteSummary>,
    pub active_products: i64,
    pub follow_ups: Vec<FollowUp>,
}

/// A cached answer and how old it is, so the UI can say "updated 2 min ago".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cached<T> {
    pub value: T,
    /// Unix seconds when it was fetched.
    pub fetched_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_cross_ipc_as_kind_and_message() -> Result<(), serde_json::Error> {
        let e: ApiError = serde_json::from_str(r#"{"kind":"forbidden","message":"Admins only"}"#)?;
        assert_eq!(e, ApiError::Forbidden("Admins only".into()));
        assert_eq!(serde_json::to_string(&ApiError::SignedOut)?, r#"{"kind":"signed_out"}"#);
        Ok(())
    }
}
