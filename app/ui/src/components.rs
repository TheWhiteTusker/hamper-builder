//! Shared building blocks, styled as in the Next app (`components/ui`).

use crate::state::{Auth, Connection};
use crate::tauri;
use lattice_core::records::{Profile, UserRole};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

#[component]
pub fn PageHeader(
    #[prop(into)] title: Signal<String>,
    #[prop(optional, into)] subtitle: Option<Signal<String>>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="no-print mb-5 flex flex-wrap items-end justify-between gap-3">
            <div>
                <h1 class="text-2xl font-semibold tracking-tight">{title}</h1>
                {subtitle.map(|s| view! { <p class="mt-0.5 text-sm text-(--color-muted)">{s}</p> })}
            </div>
            <div class="flex flex-wrap items-center gap-2">{children()}</div>
        </div>
    }
}

#[component]
pub fn Stat(
    #[prop(into)] label: String,
    #[prop(into)] value: Signal<String>,
    #[prop(optional, into)] hint: Option<String>,
) -> impl IntoView {
    view! {
        <div class="card p-4">
            <div class="label">{label}</div>
            <div class="mt-1 text-2xl font-semibold tabular-nums">{value}</div>
            {hint.map(|h| view! { <div class="mt-0.5 text-xs text-(--color-muted)">{h}</div> })}
        </div>
    }
}

#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(optional, into)] hint: Option<String>,
    action: Option<(&'static str, &'static str)>,
) -> impl IntoView {
    view! {
        <div class="card flex flex-col items-center gap-2 p-10 text-center">
            <p class="font-medium">{title}</p>
            {hint.map(|h| view! { <p class="max-w-md text-sm text-(--color-muted)">{h}</p> })}
            {action.map(|(href, label)| view! { <A href=href attr:class="btn-primary mt-2">{label}</A> })}
        </div>
    }
}

/// Shown while the last refresh could not reach the server; data on screen is the saved copy.
#[component]
pub fn OfflineBanner() -> impl IntoView {
    let offline = expect_context::<Connection>().0;
    view! {
        <Show when=move || offline.get()>
            <div role="status" class="no-print bg-amber-100 px-4 py-1.5 text-center text-xs text-amber-900">
                "Offline: showing the last saved copy. Changes need a connection to save."
            </div>
        </Show>
    }
}

fn role_label(role: UserRole) -> &'static str {
    match role {
        UserRole::Admin => "Admin",
        UserRole::Manager => "Manager",
        UserRole::Sales => "Sales",
    }
}

/// The sage top bar: logo, sections (Settings for admins), who is signed in, sign out.
#[component]
pub fn Nav(profile: Profile) -> impl IntoView {
    let auth = expect_context::<Auth>();
    let mut links = vec![
        ("/dashboard", "Dashboard"),
        ("/hampers", "Hampers"),
        ("/presentations", "Presentations"),
        ("/quotes", "Quotes"),
        ("/clients", "Clients"),
        ("/products", "Products"),
    ];
    if profile.role.is_admin() {
        links.push(("/settings", "Settings"));
    }
    let name = profile.full_name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "Account".into());
    let title = name.clone();
    let sign_out = move |_| {
        spawn_local(async move {
            let _ = tauri::call::<()>("sign_out").await;
            auth.signed_out();
        });
    };

    view! {
        <header class="no-print bg-(--color-brand)">
            <div class="flex flex-wrap items-center justify-between gap-[clamp(0.5rem,1.4vw,1.5rem)] px-[clamp(0.75rem,1.8vw,1.5rem)] py-[clamp(0.5rem,0.9vw,0.75rem)]">
                <A href="/dashboard" attr:aria-label="Lattice Lane, dashboard" attr:class="shrink-0">
                    <img src="/assets/lattice-lane-logo.png" alt="Lattice Lane" class="h-[clamp(1rem,1.6vw,1.75rem)] w-auto" />
                </A>
                <nav class="flex min-w-0 flex-1 flex-wrap items-center gap-[clamp(0.1rem,0.3vw,0.25rem)]">
                    {links
                        .into_iter()
                        .map(|(href, label)| {
                            view! {
                                <A
                                    href=href
                                    attr:class="shrink-0 rounded-full px-[clamp(0.4rem,0.75vw,0.75rem)] py-1.5 text-[clamp(0.75rem,0.95vw,0.875rem)] text-white/75 transition-colors hover:bg-white/10 hover:text-white aria-[current=page]:bg-paper aria-[current=page]:font-medium aria-[current=page]:text-(--color-brand-dark)"
                                >
                                    {label}
                                </A>
                            }
                        })
                        .collect_view()}
                </nav>
                <div class="flex shrink-0 items-center gap-[clamp(0.4rem,0.9vw,0.75rem)] whitespace-nowrap text-[clamp(0.75rem,0.95vw,0.875rem)]">
                    <span class="text-white/75" title=title>
                        <span class="hidden xl:inline">{name}</span>
                        <span class="rounded-full border border-white/25 bg-white/10 px-2 py-0.5 text-[11px] font-medium text-white xl:ml-2">
                            {role_label(profile.role)}
                        </span>
                    </span>
                    <button
                        type="button"
                        on:click=sign_out
                        class="btn rounded-full border border-white/30 px-[clamp(0.6rem,1vw,1rem)] text-[length:inherit] text-white hover:bg-white/10"
                    >
                        "Sign out"
                    </button>
                </div>
            </div>
        </header>
    }
}
