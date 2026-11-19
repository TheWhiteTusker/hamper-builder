//! App-wide state: who is signed in, the connection banner, and cached
//! queries (show the device's copy at once, then refresh from the network).

use crate::tauri;
use lattice_core::api::{ApiError, Cached, Query, SessionInfo};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Loading,
    SignedOut,
    /// Opened from a reset link: ask for the new password first.
    Recovering,
    SignedIn(SessionInfo),
}

#[derive(Clone, Copy)]
pub struct Auth {
    pub phase: RwSignal<Phase>,
    /// Shown on the sign-in card (e.g. a reset link that failed).
    pub notice: RwSignal<Option<String>>,
}

impl Auth {
    fn new() -> Self {
        Self { phase: RwSignal::new(Phase::Loading), notice: RwSignal::new(None) }
    }

    pub fn apply(&self, info: SessionInfo) {
        self.phase.set(match &info {
            SessionInfo { user: None, .. } => Phase::SignedOut,
            SessionInfo { recovering: true, .. } => Phase::Recovering,
            _ => Phase::SignedIn(info),
        });
    }

    /// Re-reads who is signed in from the native side (instant, no network).
    pub fn reload(self) {
        spawn_local(async move {
            match tauri::call::<SessionInfo>("session_state").await {
                Ok(info) => self.apply(info),
                Err(e) => {
                    self.notice.set(Some(e.to_string()));
                    self.phase.set(Phase::SignedOut);
                }
            }
        });
    }

    pub fn signed_out(&self) {
        self.phase.set(Phase::SignedOut);
    }
}

/// Bumped when the window regains focus: open screens refresh their data.
#[derive(Clone, Copy)]
pub struct Freshness(pub RwSignal<u32>);

/// Set while the last network refresh failed for want of a connection.
#[derive(Clone, Copy)]
pub struct Connection(pub RwSignal<bool>);

pub fn provide_app_state() -> Auth {
    let auth = Auth::new();
    provide_context(auth);
    provide_context(Freshness(RwSignal::new(0)));
    provide_context(Connection(RwSignal::new(false)));
    auth
}

pub struct QueryState<T: Send + Sync + 'static> {
    pub data: RwSignal<Option<Cached<T>>>,
    pub error: RwSignal<Option<ApiError>>,
    pub refreshing: RwSignal<bool>,
}

impl<T: Send + Sync + 'static> Clone for QueryState<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Send + Sync + 'static> Copy for QueryState<T> {}

fn typed<T: DeserializeOwned>(c: Cached<Value>) -> Option<Cached<T>> {
    serde_json::from_value(c.value).ok().map(|value| Cached { value, fetched_at: c.fetched_at })
}

/// Stale-while-revalidate: the cached copy first (no network, so instant),
/// then a fresh one, and again whenever the window regains focus.
pub fn use_query<T: DeserializeOwned + Clone + Send + Sync + 'static>(key: Query) -> QueryState<T> {
    let state = QueryState { data: RwSignal::new(None), error: RwSignal::new(None), refreshing: RwSignal::new(false) };
    let auth = expect_context::<Auth>();
    let online = expect_context::<Connection>();
    let tick = expect_context::<Freshness>();

    spawn_local(async move {
        if let Ok(Some(cached)) =
            tauri::invoke::<_, Option<Cached<Value>>>("query", &json!({ "key": key, "fresh": false })).await
        {
            state.data.set(typed(cached));
        }
    });

    Effect::new(move |_| {
        tick.0.track();
        state.refreshing.set(true);
        spawn_local(async move {
            let result =
                tauri::invoke::<_, Option<Cached<Value>>>("query", &json!({ "key": key, "fresh": true })).await;
            state.refreshing.set(false);
            match result {
                Ok(fresh) => {
                    online.0.set(false);
                    state.error.set(None);
                    if let Some(fresh) = fresh.and_then(typed) {
                        state.data.set(Some(fresh));
                    }
                }
                Err(ApiError::SignedOut) => auth.signed_out(),
                Err(e @ ApiError::Offline(_)) => {
                    online.0.set(true);
                    state.error.set(Some(e));
                }
                Err(e) => state.error.set(Some(e)),
            }
        });
    });
    state
}

/// "just now", "5 min ago", "3 h ago", "2 d ago".
pub fn age(fetched_at: i64) -> String {
    #[allow(clippy::cast_possible_truncation)]
    let now = (js_sys::Date::now() / 1000.0) as i64;
    match (now - fetched_at).max(0) {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{} min ago", s / 60),
        s if s < 86_400 => format!("{} h ago", s / 3600),
        s => format!("{} d ago", s / 86_400),
    }
}
