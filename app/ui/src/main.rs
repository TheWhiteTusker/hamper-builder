//! Lattice Lane UI, compiled to WASM and shown in the app's webview.

mod components;
mod pages;
mod state;
mod tauri;

use components::{Nav, OfflineBanner};
use lattice_core::api::{ApiError, SessionInfo};
use leptos::prelude::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;
use pages::{Coming, NotFound};
use state::{Freshness, Phase};

#[component]
fn App() -> impl IntoView {
    let auth = state::provide_app_state();
    auth.reload();

    // A reset link opened the app and signed in (or failed): re-read the session.
    tauri::listen::<Result<(), ApiError>>("session-changed", move |result| {
        if let Err(e) = result {
            auth.notice.set(Some(e.to_string()));
        }
        auth.reload();
    });

    // Coming back to the window refreshes whatever is on screen.
    let tick = expect_context::<Freshness>().0;
    let _focus = window_event_listener(leptos::ev::focus, move |_| tick.update(|n| *n += 1));

    view! {
        <Router>
            {move || match auth.phase.get() {
                Phase::Loading => view! { <div class="min-h-screen bg-black"></div> }.into_any(),
                Phase::SignedOut => view! { <pages::login::Login /> }.into_any(),
                Phase::Recovering => view! { <pages::reset::ResetPassword /> }.into_any(),
                Phase::SignedIn(info) => view! { <Shell info /> }.into_any(),
            }}
        </Router>
    }
}

#[component]
fn Shell(info: SessionInfo) -> impl IntoView {
    let profile = info.profile.unwrap_or_default();
    let dash = profile.clone();
    view! {
        <Nav profile=profile />
        <OfflineBanner />
        <main id="top" class="mx-auto w-full max-w-350 flex-1 px-4 py-6">
            <Routes fallback=|| view! { <NotFound /> }>
                <Route path=path!("/") view=|| view! { <Redirect path="/dashboard" /> } />
                <Route path=path!("/dashboard") view=move || view! { <pages::dashboard::Dashboard profile=dash.clone() /> } />
                <Route path=path!("/hampers/*any") view=|| view! { <Coming title="Hampers" phase="Phase 3" /> } />
                <Route path=path!("/quotes/*any") view=|| view! { <Coming title="Quotes" phase="Phase 3" /> } />
                <Route path=path!("/clients") view=|| view! { <Coming title="Clients" phase="Phase 3" /> } />
                <Route path=path!("/products/*any") view=|| view! { <Coming title="Products" phase="Phase 2" /> } />
                <Route path=path!("/presentations/*any") view=|| view! { <Coming title="Presentations" phase="Phase 5" /> } />
                <Route path=path!("/settings") view=|| view! { <Coming title="Settings" phase="Phase 3" /> } />
                <Route path=path!("/admin/*any") view=|| view! { <Coming title="Admin" phase="Phase 4" /> } />
            </Routes>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
