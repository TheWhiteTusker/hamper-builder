pub mod admin;
pub mod bin;
pub mod calculator;
pub mod clients;
pub mod dashboard;
pub mod hampers;
pub mod history;
pub mod keys;
pub mod landing;
pub mod login;
pub mod presentations;
pub mod product;
pub mod products;
pub mod quotes;
pub mod rates;
pub mod reset;
pub mod settings;

use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="card p-10 text-center">
            <p class="font-medium">"Page not found"</p>
            <leptos_router::components::A href="/dashboard" attr:class="btn-secondary mt-3">"Back to the dashboard"</leptos_router::components::A>
        </div>
    }
}
