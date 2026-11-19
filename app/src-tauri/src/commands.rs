//! What the UI can ask the native side to do. Each command returns
//! `Result<_, ApiError>`, which reaches the UI as `{ kind, message }`.

use crate::cache::Cache;
use crate::session::Sessions;
use crate::supabase::Supabase;
use base64::Engine as _;
use lattice_core::api::{ApiError, AuthUser, Cached, Dashboard, FollowUp, Query, SessionInfo, SignUpOutcome};
use lattice_core::records::{HamperSummary, Profile, QuoteSummary};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub api: Supabase,
    pub sessions: Sessions,
    pub cache: Cache,
    /// Signed in from a reset link: the UI asks for a new password first.
    pub recovering: AtomicBool,
}

/// Where the reset email sends people back to. Add it under Supabase →
/// Authentication → URL Configuration → Redirect URLs.
pub const AUTH_CALLBACK: &str = "latticelane://auth/callback";
/// The UI listens for this to re-read the session after a deep-link sign-in.
pub const SESSION_CHANGED: &str = "session-changed";
/// Cache scope for state that belongs to no user yet (the PKCE verifier).
const DEVICE: &str = "_device";

fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

async fn fetch_profile(state: &AppState, token: &str, user: &AuthUser) -> Result<Option<Profile>, ApiError> {
    let filter = format!("eq.{}", user.id);
    let rows: Vec<Profile> =
        state.api.select(token, "profiles", &[("select", "id,full_name,role,created_at"), ("id", &filter)]).await?;
    let profile = rows.into_iter().next();
    if let Some(p) = &profile
        && let Ok(json) = serde_json::to_string(p)
    {
        state.cache.put(&user.id, Query::Profile.key(), &json, now());
    }
    Ok(profile)
}

fn cached_profile(state: &AppState, user: &AuthUser) -> Option<Profile> {
    state.cache.get(&user.id, Query::Profile.key()).and_then(|(json, _)| serde_json::from_str(&json).ok())
}

/// Who is signed in, from memory and the cache: no network, so it is instant.
#[tauri::command(rename_all = "snake_case")]
pub async fn session_state(state: State<'_, AppState>) -> Result<SessionInfo, ApiError> {
    let user = state.sessions.user().await;
    let profile = user.as_ref().and_then(|u| cached_profile(&state, u));
    Ok(SessionInfo { user, profile, recovering: state.recovering.load(Ordering::Relaxed) })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sign_in(state: State<'_, AppState>, email: String, password: String) -> Result<SessionInfo, ApiError> {
    let session = state.api.sign_in(email.trim(), &password).await?;
    let (token, user) = (session.access_token.clone(), session.user.clone());
    state.sessions.set(session).await;
    let Some(profile) = fetch_profile(&state, &token, &user).await? else {
        state.sessions.clear().await;
        return Err(ApiError::Forbidden(
            "This account has no profile yet. Ask an admin to check the profiles table.".into(),
        ));
    };
    state.recovering.store(false, Ordering::Relaxed);
    Ok(SessionInfo { user: Some(user), profile: Some(profile), recovering: false })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sign_up(
    state: State<'_, AppState>,
    email: String,
    password: String,
    full_name: String,
) -> Result<SignUpOutcome, ApiError> {
    if full_name.trim().is_empty() {
        return Err(ApiError::Invalid("Enter your name.".into()));
    }
    match state.api.sign_up(email.trim(), &password, full_name.trim()).await? {
        Some(session) => {
            state.sessions.set(session).await;
            Ok(SignUpOutcome::SignedIn)
        }
        None => Ok(SignUpOutcome::ConfirmEmail),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sign_out(state: State<'_, AppState>) -> Result<(), ApiError> {
    if let Ok(token) = state.sessions.token(&state.api).await {
        // Best effort: the local session goes either way.
        let _ = state.api.sign_out(&token).await;
    }
    state.sessions.clear().await;
    state.recovering.store(false, Ordering::Relaxed);
    Ok(())
}

/// PKCE: a random verifier kept on this device, and its SHA-256 sent with the request.
fn pkce_pair() -> Result<(String, String), ApiError> {
    let mut bytes = [0u8; 48];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Server(format!("No secure randomness available: {e}")))?;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let verifier = b64.encode(bytes);
    let challenge = b64.encode(Sha256::digest(verifier.as_bytes()));
    Ok((verifier, challenge))
}

/// Emails a reset link. The verifier is saved to disk, so the link still
/// works if the app was closed in between.
#[tauri::command(rename_all = "snake_case")]
pub async fn request_password_reset(state: State<'_, AppState>, email: String) -> Result<(), ApiError> {
    let (verifier, challenge) = pkce_pair()?;
    state.cache.put(DEVICE, "pkce_verifier", &verifier, now());
    state.api.recover(email.trim(), &challenge, AUTH_CALLBACK).await
}

/// A `latticelane://auth/callback?code=…` link opened the app: trade the code
/// for a session, then tell the UI so it can ask for the new password.
pub async fn handle_auth_callback(app: &AppHandle, state: &AppState, url: &url::Url) {
    let code = url.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.into_owned());
    let failure = url.query_pairs().find(|(k, _)| k == "error_description").map(|(_, v)| v.into_owned());
    let result = match (code, state.cache.get(DEVICE, "pkce_verifier")) {
        (Some(code), Some((verifier, _))) => state.api.exchange_code(&code, &verifier).await,
        (Some(_), None) => Err(ApiError::Invalid("This reset link was requested from another device.".into())),
        (None, _) => {
            Err(ApiError::Invalid(failure.unwrap_or_else(|| "The reset link is invalid or has expired.".into())))
        }
    };
    let payload = match result {
        Ok(session) => {
            state.cache.remove(DEVICE, "pkce_verifier");
            state.sessions.set(session).await;
            state.recovering.store(true, Ordering::Relaxed);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("auth callback failed: {e}");
            Err(e)
        }
    };
    let _ = app.emit(SESSION_CHANGED, payload);
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_password(state: State<'_, AppState>, password: String) -> Result<(), ApiError> {
    if password.chars().count() < 8 {
        return Err(ApiError::Invalid("Use at least 8 characters.".into()));
    }
    let token = state.sessions.token(&state.api).await?;
    state.api.update_password(&token, &password).await?;
    state.recovering.store(false, Ordering::Relaxed);
    Ok(())
}

async fn dashboard(api: &Supabase, token: &str) -> Result<Dashboard, ApiError> {
    // The old app compared against the UTC date (`toISOString().slice(0, 10)`).
    let today = time::OffsetDateTime::now_utc().date().to_string();
    let due = format!("lte.{today}");
    let recent = [("select", "*"), ("order", "updated_at.desc"), ("limit", "6")];
    let active = [("select", "id"), ("is_active", "eq.true"), ("deleted_at", "is.null")];
    let due_soon = [
        ("select", "doc_no,client_name,follow_up_date,status"),
        ("follow_up_date", "not.is.null"),
        ("follow_up_date", due.as_str()),
        ("status", "not.in.(\"Won\",\"Lost\")"),
        ("order", "follow_up_date"),
        ("limit", "5"),
    ];
    let (hampers, quotes, active_products, follow_ups) = tokio::join!(
        api.select::<Vec<HamperSummary>>(token, "hamper_summary", &recent),
        api.select::<Vec<QuoteSummary>>(token, "quote_summary", &recent),
        api.count(token, "products", &active),
        api.select::<Vec<FollowUp>>(token, "quotes", &due_soon),
    );
    Ok(Dashboard { hampers: hampers?, quotes: quotes?, active_products: active_products?, follow_ups: follow_ups? })
}

/// Stale-while-revalidate: with `fresh` false, the cached copy (instant, no
/// network); with `fresh` true, a network fetch that also updates the cache.
#[tauri::command(rename_all = "snake_case")]
pub async fn query(state: State<'_, AppState>, key: Query, fresh: bool) -> Result<Option<Cached<Value>>, ApiError> {
    let user = state.sessions.user().await.ok_or(ApiError::SignedOut)?;
    if !fresh {
        return Ok(state.cache.get(&user.id, key.key()).and_then(|(json, fetched_at)| {
            serde_json::from_str(&json).ok().map(|value| Cached { value, fetched_at })
        }));
    }
    let token = state.sessions.token(&state.api).await?;
    let value = match key {
        Query::Profile => serde_json::to_value(fetch_profile(&state, &token, &user).await?),
        Query::Dashboard => serde_json::to_value(dashboard(&state.api, &token).await?),
    }
    .map_err(|e| ApiError::Server(format!("Could not read {}: {e}", key.key())))?;
    let fetched_at = now();
    if key != Query::Profile {
        state.cache.put(&user.id, key.key(), &value.to_string(), fetched_at);
    }
    Ok(Some(Cached { value, fetched_at }))
}
