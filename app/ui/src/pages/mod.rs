pub mod dashboard;
pub mod login;
pub mod reset;

use leptos::prelude::*;

/// A section still being ported, named with the phase that brings it.
#[component]
pub fn Coming(title: &'static str, phase: &'static str) -> impl IntoView {
    view! {
        <div class="card flex flex-col items-center gap-2 p-10 text-center">
            <p class="font-medium">{title}</p>
            <p class="max-w-md text-sm text-(--color-muted)">{format!("This screen arrives in {phase} of the Rust rebuild.")}</p>
        </div>
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="card p-10 text-center">
            <p class="font-medium">"Page not found"</p>
            <leptos_router::components::A href="/dashboard" attr:class="btn-secondary mt-3">"Back to the dashboard"</leptos_router::components::A>
        </div>
    }
}
