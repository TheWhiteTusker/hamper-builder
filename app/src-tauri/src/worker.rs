//! The Cloudflare Worker: only jobs that need a secret a device must not hold
//! (the GST register's API key). Called with the user's JWT, which the Worker
//! checks with Supabase before doing anything.

use crate::commands::AppState;
use lattice_core::api::ApiError;
use lattice_core::gstin::{self, GstDetails};
use serde_json::Value;
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub async fn gstin_lookup(state: State<'_, AppState>, gstin: String) -> Result<GstDetails, ApiError> {
    let value = gstin.trim().to_uppercase();
    if !gstin::is_valid(&value) {
        return Err(ApiError::Invalid("Enter a valid 15-character GSTIN first.".into()));
    }
    let Some(base) = option_env!("LATTICE_WORKER_URL").filter(|u| !u.is_empty()) else {
        return Err(ApiError::Invalid("GST lookup is not configured. Enter the name and address by hand.".into()));
    };
    let token = state.sessions.token(&state.api).await?;
    let resp = state
        .api
        .http()
        .get(format!("{}/gstin/{value}", base.trim_end_matches('/')))
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| ApiError::Offline("Could not reach the GST service. Enter the details by hand.".into()))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ApiError::SignedOut);
    }
    if !status.is_success() {
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("GST lookup failed. Enter the details by hand.");
        return Err(ApiError::Invalid(msg.to_string()));
    }
    serde_json::from_value(body).map_err(|_| ApiError::Server("GST lookup sent something unexpected.".into()))
}

/// Opens the print dialog. macOS's webview ignores `window.print()`, so it
/// prints from here; elsewhere this returns false and the page calls it.
#[tauri::command]
pub fn print_page(webview: tauri::Webview) -> bool {
    if cfg!(target_os = "macos") {
        return webview.print().is_ok();
    }
    false
}
