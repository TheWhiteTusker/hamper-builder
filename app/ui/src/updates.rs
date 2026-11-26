//! App updates (port of electron/updater.cjs): on launch, and from Settings →
//! Tools, the app asks before downloading, shows progress, then restarts on
//! the new version, so nobody reinstalls by hand. Releases come from
//! .github/workflows/desktop.yml.

use crate::tauri;
use leptos::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Available {
    version: String,
}

#[derive(Clone, Copy)]
pub struct Updates {
    found: RwSignal<Option<Available>>,
    asking: RwSignal<bool>,
    progress: RwSignal<Option<u32>>,
    checking: RwSignal<bool>,
    status: RwSignal<Option<String>>,
}

impl Updates {
    /// Once, for the life of the window: listens for download progress and
    /// checks quietly at launch.
    pub fn provide() {
        let me = Self {
            found: RwSignal::new(None),
            asking: RwSignal::new(false),
            progress: RwSignal::new(None),
            checking: RwSignal::new(false),
            status: RwSignal::new(None),
        };
        provide_context(me);
        tauri::listen::<u32>("update-progress", move |p| me.progress.set(Some(p)));
        me.check(true);
    }

    /// `quiet`: say nothing unless there's an update (the launch check).
    fn check(self, quiet: bool) {
        self.checking.set(true);
        self.status.set(None);
        leptos::task::spawn_local(async move {
            let result = tauri::call::<Option<Available>>("update_check").await;
            self.checking.set(false);
            match result {
                Ok(Some(a)) => {
                    self.found.set(Some(a));
                    self.asking.set(true);
                }
                Ok(None) if !quiet => self.status.set(Some("You're on the latest version.".into())),
                Err(e) if !quiet => self.status.set(Some(e.to_string())),
                _ => {}
            }
        });
    }

    fn install(self) {
        self.progress.set(Some(0));
        leptos::task::spawn_local(async move {
            // Success restarts the app, so only a failure comes back.
            if let Err(e) = tauri::call::<()>("update_install").await {
                self.progress.set(None);
                self.asking.set(false);
                self.status.set(Some(e.to_string()));
            }
        });
    }
}

/// The "update ready" question and the download's progress, over any screen.
#[component]
pub fn UpdatePrompt() -> impl IntoView {
    let u = expect_context::<Updates>();
    let version = move || u.found.with(|f| f.as_ref().map(|a| a.version.clone()).unwrap_or_default());
    view! {
        <Show when=move || u.asking.get()>
            <div role="dialog" aria-modal="true" aria-labelledby="update-title" class="fixed inset-0 z-50 grid place-items-center bg-black/50 p-4">
                <div class="w-[min(92vw,26rem)] rounded-xl bg-white p-5 shadow-2xl">
                    {move || match u.progress.get() {
                        None => view! {
                            <h2 id="update-title" class="text-base font-semibold">{move || format!("Lattice Lane {} is available", version())}</h2>
                            <p class="mt-2 text-sm text-(--color-muted)">"Update now downloads it and restarts the app on the new version. Later asks again the next time you open the app."</p>
                            <div class="mt-5 flex justify-end gap-2">
                                <button type="button" class="btn-secondary" on:click=move |_| u.asking.set(false)>"Later"</button>
                                <button type="button" class="btn-primary" on:click=move |_| u.install()>"Update now"</button>
                            </div>
                        }.into_any(),
                        Some(p) => view! {
                            <h2 id="update-title" class="text-base font-semibold">{move || format!("Updating to {}…", version())}</h2>
                            <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-100" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow=p>
                                <div class="h-full bg-(--color-brand) transition-[width]" style=format!("width: {p}%")></div>
                            </div>
                            <p class="mt-2 text-xs text-(--color-muted)">{format!("{p}% · the app restarts when it's done")}</p>
                        }.into_any(),
                    }}
                </div>
            </div>
        </Show>
    }
}

/// Settings → Tools: this version, and a check for a newer one.
#[component]
pub fn UpdateRow() -> impl IntoView {
    let u = expect_context::<Updates>();
    view! {
        <div class="flex flex-wrap items-center justify-between gap-3 p-4">
            <div>
                <h2 class="text-sm font-semibold">"App"</h2>
                <p class="mt-0.5 text-xs text-(--color-muted)">
                    "Lattice Lane Hamper Builder · Version "<span class="font-mono font-medium text-(--color-brand)">{concat!("v", env!("CARGO_PKG_VERSION"))}</span>
                    {move || u.status.get().map(|s| view! { <span role="status" class="ml-2">{format!("· {s}")}</span> })}
                </p>
            </div>
            <button type="button" class="btn-secondary" disabled=move || u.checking.get() on:click=move |_| u.check(false)>
                {move || if u.checking.get() { "Checking…" } else { "Check for updates" }}
            </button>
        </div>
    }
}
