//! Lattice Lane UI, compiled to WASM and shown in the app's webview.

mod components;
mod data;
mod db;
mod kit;
mod pages;
mod photos;
mod state;
mod studio;
mod tauri;

use components::{Nav, OfflineBanner};
use lattice_core::api::{ApiError, SessionInfo};
use leptos::prelude::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;
use pages::{Coming, NotFound, admin, bin, clients, hampers, quotes, settings};
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
                <Route path=path!("/hampers") view=hampers::Hampers />
                <Route path=path!("/hampers/new") view=hampers::NewHamper />
                <Route path=path!("/hampers/:code/edit") view=hampers::EditHamper />
                <Route path=path!("/hampers/:code/image") view=hampers::HamperImage />
                <Route path=path!("/hampers/:code") view=hampers::HamperView />
                <Route path=path!("/quotes") view=quotes::Quotes />
                <Route path=path!("/quotes/new") view=quotes::NewQuote />
                <Route path=path!("/quotes/:doc/print") view=quotes::PrintQuote />
                <Route path=path!("/quotes/:doc") view=quotes::QuotePage />
                <Route path=path!("/clients") view=clients::Clients />
                <Route path=path!("/products") view=pages::products::Products />
                <Route path=path!("/products/new") view=pages::calculator::NewProduct />
                <Route path=path!("/products/*code") view=pages::product::ProductView />
                <Route path=path!("/cost-calculator") view=|| view! { <Redirect path="/products/new" /> } />
                <Route path=path!("/cost-calculator/master") view=|| view! { <pages::rates::RatesMaster /> } />
                <Route path=path!("/cost-calculator/master/excel/:mode") view=admin::RateSheetPage />
                <Route path=path!("/cost-calculator/*code") view=pages::calculator::ProductCostingPage />
                <Route path=path!("/presentations/*any") view=|| view! { <Coming title="Presentations" phase="Phase 5" /> } />
                <Route path=path!("/settings") view=settings::SettingsPage />
                <Route path=path!("/admin/import") view=admin::ImportWorkbook />
                <Route path=path!("/admin/refresh-prices") view=admin::RefreshPrices />
                <Route path=path!("/admin/products/export") view=admin::ProductsExport />
                <Route path=path!("/admin/products/:mode") view=admin::ProductSheetPage />
                <Route path=path!("/bin") view=bin::Bin />
            </Routes>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
