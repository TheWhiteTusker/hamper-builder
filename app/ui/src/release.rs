//! Settings → Tools → "Update the app" (port of `update-app-button.tsx`):
//! starts the desktop build on GitHub through the Worker and shows its
//! progress. Everyone's app then offers the update on its next launch.

use crate::tauri;
use leptos::prelude::*;
use serde::Deserialize;
use std::time::Duration;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
struct Status {
    state: String,
    percent: f64,
    step: Option<String>,
    url: Option<String>,
    created_at: Option<String>,
}

fn step_label(step: Option<&str>) -> &'static str {
    match step {
        Some("Install Trunk and the Tauri CLI") => "Installing build tools",
        Some("Set version") => "Preparing",
        Some(s) if s.contains("tauri-action") => "Building and publishing",
        _ => "Preparing",
    }
}

#[component]
pub fn PublishRelease() -> impl IntoView {
    let status = RwSignal::new(None::<Status>);
    let error = RwSignal::new(None::<String>);
    let starting = RwSignal::new(false);
    // When this screen started a release (ms, less 30 s for clock skew with
    // GitHub). Runs created before it are an earlier release, not this click's.
    let since = RwSignal::new(None::<f64>);
    let fresh =
        move |s: &Status| since.get().zip(s.created_at.as_deref()).is_some_and(|(t, c)| js_sys::Date::parse(c) >= t);
    let alive = StoredValue::new(true);
    on_cleanup(move || alive.set_value(false));

    // Polls while a release runs (one started elsewhere too), and while
    // waiting for a just-started run to appear.
    fn poll(status: RwSignal<Option<Status>>, since: RwSignal<Option<f64>>, alive: StoredValue<bool>) {
        leptos::task::spawn_local(async move {
            let s = tauri::call::<Status>("release_status").await.ok();
            if !alive.get_value() {
                return;
            }
            let created = s.as_ref().and_then(|s| s.created_at.as_deref()).map(js_sys::Date::parse);
            let waiting = since.get_untracked().is_some_and(|t| created.is_none_or(|c| c < t));
            let running = s.as_ref().is_some_and(|s| s.state == "running");
            status.set(s);
            if running || waiting {
                set_timeout(move || poll(status, since, alive), Duration::from_secs(5));
            }
        });
    }
    poll(status, since, alive);

    let start = move |_| {
        starting.set(true);
        error.set(None);
        leptos::task::spawn_local(async move {
            match tauri::call::<serde_json::Value>("release_start").await {
                Ok(_) => {
                    since.set(Some(js_sys::Date::now() - 30_000.0));
                    poll(status, since, alive);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            starting.set(false);
        });
    };

    let running = move || status.with(|s| s.as_ref().is_some_and(|s| s.state == "running"));
    let waiting = move || since.get().is_some() && !status.with(|s| s.as_ref().is_some_and(fresh));
    view! {
        <div class="flex flex-wrap items-center gap-2.5">
            <button type="button" class="btn-secondary" disabled=move || starting.get() || running() || waiting() on:click=start
                title="Build the Windows app from main on GitHub and publish it">
                {move || if starting.get() { "Starting…" } else { "Update the app" }}
            </button>
            {move || (waiting() && !running()).then(|| view! { <span class="text-xs text-(--color-muted)">"Starting on GitHub…"</span> })}
            {move || status.get().filter(|s| s.state == "running").map(|s| view! {
                <div class="flex items-center gap-2" role="status" aria-live="polite">
                    <div class="h-2 w-32 overflow-hidden rounded bg-slate-200">
                        <div class="h-2 bg-(--color-brand) transition-[width] duration-1000" style=format!("width: {}%", s.percent)></div>
                    </div>
                    <span class="text-xs text-(--color-muted)">{format!("Packaging {}% · {}", s.percent, step_label(s.step.as_deref()))}</span>
                </div>
            })}
            {move || status.get().filter(|s| fresh(s)).and_then(|s| match s.state.as_str() {
                "success" => Some(view! { <span class="text-sm text-green-800">"Released. Everyone’s app will offer the update."</span> }.into_any()),
                "failure" => Some(view! {
                    <a href=s.url.unwrap_or_default() target="_blank" rel="noreferrer" class="text-sm text-red-700 underline">"Packaging failed. See the log on GitHub"</a>
                }.into_any()),
                _ => None,
            })}
            {move || error.get().map(|e| view! { <span role="alert" class="text-sm text-red-700">{e}</span> })}
        </div>
    }
}
