//! The database from the UI, shaped like the supabase-js calls the Next app
//! made, so its server actions port line for line. Everything runs through
//! the native side with the signed-in user's JWT; RLS decides each call.

use crate::state::{Auth, Connection, Freshness};
use crate::tauri;
use lattice_core::api::{ApiError, Cached};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// A PostgREST query: a table or view plus filter, order and limit params.
#[derive(Clone, Debug, Default)]
pub struct Q {
    table: String,
    params: Vec<(String, String)>,
}

/// PostgREST quotes a value in `in.(…)` / `or=(…)` lists when it has commas or brackets.
fn quoted(v: &str) -> String {
    if v.contains([',', '(', ')', '"', ' ', '.']) { format!("\"{}\"", v.replace('"', "\\\"")) } else { v.to_string() }
}

impl Q {
    pub fn from(table: &str) -> Self {
        Self { table: table.into(), params: Vec::new() }
    }
    fn p(mut self, k: &str, v: String) -> Self {
        self.params.push((k.into(), v));
        self
    }
    pub fn select(self, cols: &str) -> Self {
        self.p("select", cols.into())
    }
    pub fn eq(self, col: &str, v: impl std::fmt::Display) -> Self {
        self.p(col, format!("eq.{v}"))
    }
    pub fn neq(self, col: &str, v: impl std::fmt::Display) -> Self {
        self.p(col, format!("neq.{v}"))
    }
    pub fn lte(self, col: &str, v: impl std::fmt::Display) -> Self {
        self.p(col, format!("lte.{v}"))
    }
    pub fn null(self, col: &str) -> Self {
        self.p(col, "is.null".into())
    }
    pub fn not_null(self, col: &str) -> Self {
        self.p(col, "not.is.null".into())
    }
    pub fn ilike(self, col: &str, pattern: &str) -> Self {
        self.p(col, format!("ilike.{pattern}"))
    }
    pub fn in_list<S: AsRef<str>>(self, col: &str, values: &[S]) -> Self {
        let list: Vec<String> = values.iter().map(|v| quoted(v.as_ref())).collect();
        self.p(col, format!("in.({})", list.join(",")))
    }
    pub fn order(self, col: &str) -> Self {
        self.order_by(col, true)
    }
    pub fn order_desc(self, col: &str) -> Self {
        self.order_by(col, false)
    }
    fn order_by(mut self, col: &str, asc: bool) -> Self {
        let term = format!("{col}.{}", if asc { "asc" } else { "desc" });
        match self.params.iter_mut().find(|(k, _)| k == "order") {
            Some((_, v)) => {
                v.push(',');
                v.push_str(&term);
            }
            None => self.params.push(("order".into(), term)),
        }
        self
    }
    pub fn limit(self, n: usize) -> Self {
        self.p("limit", n.to_string())
    }
    /// Skips the first `n` rows (paging past the server's 1000-row cap).
    pub fn offset(self, n: usize) -> Self {
        self.p("offset", n.to_string())
    }

    fn args(&self, cache: Option<&str>, fresh: bool) -> Value {
        json!({ "table": self.table, "query": self.params, "cache": cache, "fresh": fresh })
    }

    /// All matching rows, from the network.
    pub async fn rows<T: DeserializeOwned>(&self) -> Result<Vec<T>, ApiError> {
        let got: Option<Cached<Vec<T>>> = tauri::invoke("db_select", &self.args(None, true)).await?;
        Ok(got.map(|c| c.value).unwrap_or_default())
    }

    /// The first matching row, if any.
    pub async fn one<T: DeserializeOwned>(&self) -> Result<Option<T>, ApiError> {
        Ok(self.clone().limit(1).rows::<T>().await?.into_iter().next())
    }

    pub async fn count(&self) -> Result<i64, ApiError> {
        tauri::invoke("db_count", &json!({ "table": self.table, "query": self.params })).await
    }

    /// `update … where <filters>`; returns the updated rows when `select` is set.
    pub async fn update<P: Serialize>(&self, patch: &P, select: Option<&str>) -> Result<Value, ApiError> {
        tauri::invoke(
            "db_update",
            &json!({ "table": self.table, "query": self.params, "patch": patch, "select": select }),
        )
        .await
    }

    pub async fn delete(&self) -> Result<Value, ApiError> {
        tauri::invoke("db_delete", &json!({ "table": self.table, "query": self.params })).await
    }
}

/// `insert` one row or many; with `on_conflict`, an upsert on those columns.
pub async fn insert<R: Serialize>(
    table: &str,
    rows: &R,
    on_conflict: Option<&str>,
    select: Option<&str>,
) -> Result<Value, ApiError> {
    tauri::invoke("db_insert", &json!({ "table": table, "rows": rows, "on_conflict": on_conflict, "select": select }))
        .await
}

/// Inserted rows read back as `T` (insert with `select=*`).
pub async fn insert_returning<R: Serialize, T: DeserializeOwned>(table: &str, rows: &R) -> Result<Vec<T>, ApiError> {
    let v = insert(table, rows, None, Some("*")).await?;
    serde_json::from_value(v).map_err(|e| ApiError::Server(format!("Unexpected rows from {table}: {e}")))
}

pub async fn rpc<A: Serialize, R: DeserializeOwned>(name: &str, args: &A) -> Result<R, ApiError> {
    let v: Value = tauri::invoke("db_rpc", &json!({ "name": name, "args": args })).await?;
    serde_json::from_value(v).map_err(|e| ApiError::Server(format!("Unexpected result from {name}: {e}")))
}

/// Deletes stored files (a no-op for none).
pub async fn remove_files(paths: &[String]) -> Result<(), ApiError> {
    if paths.is_empty() {
        return Ok(());
    }
    tauri::invoke("storage_remove", &json!({ "bucket": PHOTOS, "paths": paths })).await
}

/// The files directly inside `prefix`, as full paths (`products/{id}/a.webp`).
pub async fn list_files(prefix: &str) -> Result<Vec<String>, ApiError> {
    let v: Value = tauri::invoke("storage_list", &json!({ "bucket": PHOTOS, "prefix": prefix })).await?;
    Ok(v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.get("name").and_then(Value::as_str))
        .map(|n| format!("{prefix}/{n}"))
        .collect())
}

pub const PHOTOS: &str = "product-images";
/// Uploaded objects never change in place (new names each time), so cache them a year.
pub const IMMUTABLE: &str = "31536000";

/// Uploads bytes the UI made (a resized photo, a rendered design) and returns the public URL.
pub async fn upload(
    bucket: &str,
    path: &str,
    bytes: &[u8],
    content_type: &str,
    upsert: bool,
) -> Result<String, ApiError> {
    tauri::invoke_raw(
        "storage_upload",
        bytes,
        &[
            ("x-bucket", bucket),
            ("x-path", path),
            ("content-type", content_type),
            ("x-cache-control", IMMUTABLE),
            ("x-upsert", if upsert { "true" } else { "false" }),
        ],
    )
    .await
}

/// Replaces a cached copy after a save, so other screens show it at once.
pub fn cache_put<T: Serialize>(key: &str, value: &T) {
    let args = json!({ "key": key, "value": value });
    spawn_local(async move {
        let _ = tauri::invoke::<_, ()>("cache_put", &args).await;
    });
}

/// Forgets cached copies after a save. Awaited before navigating, so the
/// next screen can't read the stale copy first.
pub async fn cache_drop(keys: &[&str]) {
    let _ = tauri::invoke::<_, ()>("cache_drop", &json!({ "keys": keys })).await;
}

/// A cached query's state on screen.
pub struct Live<T: Send + Sync + 'static> {
    pub data: RwSignal<Option<T>>,
    pub fetched_at: RwSignal<Option<i64>>,
    pub error: RwSignal<Option<ApiError>>,
    pub refreshing: RwSignal<bool>,
    reload: RwSignal<u32>,
}

impl<T: Send + Sync + 'static> Clone for Live<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Send + Sync + 'static> Copy for Live<T> {}

impl<T: Send + Sync + 'static> Live<T> {
    /// Shows a change straight away, before the server confirms it.
    pub fn patch(&self, f: impl FnOnce(&mut T)) {
        self.data.update(|d| {
            if let Some(d) = d {
                f(d);
            }
        });
    }
}

fn typed<T: DeserializeOwned>(c: Cached<Value>) -> Option<(T, i64)> {
    serde_json::from_value(c.value).ok().map(|v| (v, c.fetched_at))
}

/// Rows of `q`, shown from the device's copy under `key` at once, then
/// refreshed from the network, and again whenever the window regains focus.
pub fn live<T>(key: impl Into<String>, q: Q) -> Live<Vec<T>>
where
    T: DeserializeOwned + Clone + Send + Sync + 'static,
{
    let key: String = key.into();
    let state = Live {
        data: RwSignal::new(None),
        fetched_at: RwSignal::new(None),
        error: RwSignal::new(None),
        refreshing: RwSignal::new(false),
        reload: RwSignal::new(0),
    };
    let auth = expect_context::<Auth>();
    let offline = expect_context::<Connection>().0;
    let tick = expect_context::<Freshness>();

    let cached_args = q.args(Some(&key), false);
    spawn_local(async move {
        if let Ok(Some(c)) = tauri::invoke::<_, Option<Cached<Value>>>("db_select", &cached_args).await
            && let Some((rows, at)) = typed::<Vec<T>>(c)
            && state.data.get_untracked().is_none()
        {
            state.data.set(Some(rows));
            state.fetched_at.set(Some(at));
        }
    });

    let fresh_args = q.args(Some(&key), true);
    Effect::new(move |_| {
        tick.0.track();
        state.reload.track();
        state.refreshing.set(true);
        let args = fresh_args.clone();
        spawn_local(async move {
            let result = tauri::invoke::<_, Option<Cached<Value>>>("db_select", &args).await;
            state.refreshing.set(false);
            match result {
                Ok(Some(c)) => {
                    offline.set(false);
                    state.error.set(None);
                    if let Some((rows, at)) = typed::<Vec<T>>(c) {
                        state.data.set(Some(rows));
                        state.fetched_at.set(Some(at));
                    }
                }
                Ok(None) => {}
                Err(ApiError::SignedOut) => auth.signed_out(),
                Err(e @ ApiError::Offline(_)) => {
                    offline.set(true);
                    state.error.set(Some(e));
                }
                Err(e) => state.error.set(Some(e)),
            }
        });
    });
    state
}

/// Like [`live`] for anything `fetch` can build (several queries joined, say):
/// the device's copy under `key` first, then `fetch`'s result, which replaces it.
pub fn live_with<T, F, Fut>(key: impl Into<String>, fetch: F) -> Live<T>
where
    T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
    F: Fn() -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, ApiError>> + 'static,
{
    let key: String = key.into();
    let state = Live {
        data: RwSignal::new(None),
        fetched_at: RwSignal::new(None),
        error: RwSignal::new(None),
        refreshing: RwSignal::new(false),
        reload: RwSignal::new(0),
    };
    let auth = expect_context::<Auth>();
    let offline = expect_context::<Connection>().0;
    let tick = expect_context::<Freshness>();

    let cached_key = key.clone();
    spawn_local(async move {
        if let Ok(Some(c)) = tauri::invoke::<_, Option<Cached<Value>>>("cache_get", &json!({ "key": cached_key })).await
            && let Some((value, at)) = typed::<T>(c)
            && state.data.get_untracked().is_none()
        {
            state.data.set(Some(value));
            state.fetched_at.set(Some(at));
        }
    });

    let fetch = std::rc::Rc::new(fetch);
    Effect::new(move |_| {
        tick.0.track();
        state.reload.track();
        state.refreshing.set(true);
        let (fetch, key) = (fetch.clone(), key.clone());
        spawn_local(async move {
            let result = fetch().await;
            state.refreshing.set(false);
            match result {
                Ok(value) => {
                    offline.set(false);
                    state.error.set(None);
                    cache_put(&key, &value);
                    state.fetched_at.set(Some(crate::state::now()));
                    state.data.set(Some(value));
                }
                Err(ApiError::SignedOut) => auth.signed_out(),
                Err(e @ ApiError::Offline(_)) => {
                    offline.set(true);
                    state.error.set(Some(e));
                }
                Err(e) => state.error.set(Some(e)),
            }
        });
    });
    state
}

/// The signed-in profile, for role checks inside screens.
pub fn me() -> lattice_core::records::Profile {
    match expect_context::<Auth>().phase.get_untracked() {
        crate::state::Phase::SignedIn(info) => info.profile.unwrap_or_default(),
        _ => lattice_core::records::Profile::default(),
    }
}
