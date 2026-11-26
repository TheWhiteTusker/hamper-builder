//! The database and storage, for the UI: the same operations the Next app's
//! server actions made through supabase-js, with the signed-in user's JWT, so
//! row level security decides each one. Reads can be cached on the device by
//! key, so screens open with the last copy instantly.

use crate::commands::AppState;
use lattice_core::api::{ApiError, Cached};
use serde_json::Value;
use tauri::State;

type Params = Vec<(String, String)>;

fn pairs(p: &Params) -> Vec<(&str, &str)> {
    p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

async fn token(state: &AppState) -> Result<String, ApiError> {
    state.sessions.token(&state.api).await
}

async fn scope(state: &AppState) -> Result<String, ApiError> {
    state.sessions.user().await.map(|u| u.id).ok_or(ApiError::SignedOut)
}

/// Rows from a table or view. With a `cache` key: `fresh = false` answers from
/// the device only (instant), `fresh = true` fetches and updates the copy.
#[tauri::command(rename_all = "snake_case")]
pub async fn db_select(
    state: State<'_, AppState>,
    table: String,
    query: Params,
    cache: Option<String>,
    fresh: bool,
) -> Result<Option<Cached<Value>>, ApiError> {
    let scope = scope(&state).await?;
    if let (Some(key), false) = (&cache, fresh) {
        return Ok(state.cache.get(&scope, key).and_then(|(json, fetched_at)| {
            serde_json::from_str(&json).ok().map(|value| Cached { value, fetched_at })
        }));
    }
    let token = token(&state).await?;
    let value: Value = state.api.select(&token, &table, &pairs(&query)).await?;
    let fetched_at = now();
    if let Some(key) = cache {
        state.cache.put(&scope, &key, &value.to_string(), fetched_at);
    }
    Ok(Some(Cached { value, fetched_at }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn db_count(state: State<'_, AppState>, table: String, query: Params) -> Result<i64, ApiError> {
    let token = token(&state).await?;
    state.api.count(&token, &table, &pairs(&query)).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn db_insert(
    state: State<'_, AppState>,
    table: String,
    rows: Value,
    on_conflict: Option<String>,
    select: Option<String>,
) -> Result<Value, ApiError> {
    let token = token(&state).await?;
    state.api.insert(&token, &table, &rows, on_conflict.as_deref(), select.as_deref()).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn db_update(
    state: State<'_, AppState>,
    table: String,
    query: Params,
    patch: Value,
    select: Option<String>,
) -> Result<Value, ApiError> {
    if query.is_empty() {
        return Err(ApiError::Invalid("Refusing to update every row: no filter given.".into()));
    }
    let token = token(&state).await?;
    state.api.update(&token, &table, &pairs(&query), &patch, select.as_deref()).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn db_delete(state: State<'_, AppState>, table: String, query: Params) -> Result<Value, ApiError> {
    if query.is_empty() {
        return Err(ApiError::Invalid("Refusing to delete every row: no filter given.".into()));
    }
    let token = token(&state).await?;
    state.api.delete(&token, &table, &pairs(&query)).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn db_rpc(state: State<'_, AppState>, name: String, args: Value) -> Result<Value, ApiError> {
    let token = token(&state).await?;
    state.api.rpc(&token, &name, &args).await
}

/// A cached copy by key, without touching the network.
#[tauri::command(rename_all = "snake_case")]
pub async fn cache_get(state: State<'_, AppState>, key: String) -> Result<Option<Cached<Value>>, ApiError> {
    let scope = scope(&state).await?;
    Ok(state
        .cache
        .get(&scope, &key)
        .and_then(|(json, fetched_at)| serde_json::from_str(&json).ok().map(|value| Cached { value, fetched_at })))
}

/// Replaces a cached copy after a save, so other screens show it at once.
#[tauri::command(rename_all = "snake_case")]
pub async fn cache_put(state: State<'_, AppState>, key: String, value: Value) -> Result<(), ApiError> {
    let scope = scope(&state).await?;
    state.cache.put(&scope, &key, &value.to_string(), now());
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn cache_drop(state: State<'_, AppState>, keys: Vec<String>) -> Result<(), ApiError> {
    let scope = scope(&state).await?;
    for key in keys {
        state.cache.remove(&scope, &key);
    }
    Ok(())
}

fn header<'a>(request: &'a tauri::ipc::Request<'_>, name: &str) -> Option<&'a str> {
    request.headers().get(name).and_then(|v| v.to_str().ok())
}

/// Stores bytes the UI produced (a resized photo, a rendered design) without
/// sending them as JSON: the body is raw, the details are headers.
#[tauri::command]
pub async fn storage_upload(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> Result<String, ApiError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Invalid("Upload body must be raw bytes.".into()));
    };
    let bucket = header(&request, "x-bucket").ok_or_else(|| ApiError::Invalid("No bucket given.".into()))?;
    let path = header(&request, "x-path").ok_or_else(|| ApiError::Invalid("No path given.".into()))?;
    if path.contains("..") || path.starts_with('/') {
        return Err(ApiError::Invalid("That storage path is not allowed.".into()));
    }
    let content_type = header(&request, "content-type").unwrap_or("application/octet-stream");
    let cache_control = header(&request, "x-cache-control").unwrap_or("3600");
    let upsert = header(&request, "x-upsert") == Some("true");
    let token = token(&state).await?;
    let upload = crate::supabase::Upload { bucket, path, content_type, cache_control, upsert };
    state.api.upload(&token, &upload, bytes.clone()).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn storage_remove(state: State<'_, AppState>, bucket: String, paths: Vec<String>) -> Result<(), ApiError> {
    let token = token(&state).await?;
    state.api.remove(&token, &bucket, &paths).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn storage_list(state: State<'_, AppState>, bucket: String, prefix: String) -> Result<Value, ApiError> {
    let token = token(&state).await?;
    state.api.list(&token, &bucket, &prefix).await
}

#[tauri::command(rename_all = "snake_case")]
pub fn storage_public_url(state: State<'_, AppState>, bucket: String, path: String) -> String {
    state.api.public_url(&bucket, &path)
}
