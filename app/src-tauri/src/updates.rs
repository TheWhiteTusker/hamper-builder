//! App updates: signed installers that .github/workflows/desktop.yml publishes
//! to GitHub Releases. The UI asks before downloading (settings and launch).

use lattice_core::api::ApiError;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

/// The update the last check found, waiting for the person to say yes.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

#[derive(Serialize)]
pub struct Available {
    version: String,
    notes: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_check(app: AppHandle, pending: State<'_, Pending>) -> Result<Option<Available>, ApiError> {
    let fail = |e: tauri_plugin_updater::Error| ApiError::Offline(format!("Could not check for updates: {e}"));
    let update = app.updater().map_err(fail)?.check().await.map_err(fail)?;
    let found = update.as_ref().map(|u| Available { version: u.version.clone(), notes: u.body.clone() });
    *pending.0.lock().await = update;
    Ok(found)
}

/// Downloads the update `update_check` found, sending "update-progress"
/// (0–100) as it goes, installs it and restarts on the new version.
#[tauri::command(rename_all = "snake_case")]
pub async fn update_install(app: AppHandle, pending: State<'_, Pending>) -> Result<(), ApiError> {
    let Some(update) = pending.0.lock().await.take() else {
        return Err(ApiError::Invalid("There's no update waiting. Check for updates first.".into()));
    };
    let progress = app.clone();
    let mut got: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                got += u64::try_from(chunk).unwrap_or(0);
                if let Some(total) = total.filter(|t| *t > 0) {
                    let _ = progress.emit("update-progress", (got * 100 / total).min(100));
                }
            },
            || {},
        )
        .await
        .map_err(|e| ApiError::Server(format!("The update could not be installed: {e}")))?;
    tracing::info!("updated to {}; restarting", update.version);
    app.restart()
}
