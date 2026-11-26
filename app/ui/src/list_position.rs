//! A list's last URL (filters, rows shown) and scroll spot, so "← Products" /
//! "← Hampers" and saving a hamper land where you left off (port of
//! `list-position.tsx`). Kept for the window's session.

use lattice_core::list_return::list_return;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

#[derive(Default, Deserialize, Serialize)]
struct Spot {
    url: String,
    y: i32,
}

fn session() -> Option<web_sys::Storage> {
    window().session_storage().ok().flatten()
}

fn load(kind: &str) -> Option<Spot> {
    session()?.get_item(&format!("lattice:list-{kind}")).ok()?.and_then(|raw| serde_json::from_str(&raw).ok())
}

/// The rows scroll inside this box (app.css), not the window.
fn scroll_box() -> Option<web_sys::Element> {
    document().query_selector(".list-scroll").ok().flatten()
}

fn here() -> String {
    let l = window().location();
    format!("{}{}", l.pathname().unwrap_or_default(), l.search().unwrap_or_default())
}

/// The list as it was left: its filters.
pub fn list_href(kind: &str) -> String {
    load(kind).map_or_else(|| format!("/{kind}"), |s| s.url)
}

/// Where a save (or, with `deleted`, a move to the Bin) lands.
pub fn after_save(kind: &str, deleted: bool) -> String {
    list_return(kind, &list_href(kind), deleted)
}

/// The next visit to the list scrolls back to the remembered spot.
pub fn mark_restore(kind: &str) {
    if let Some(s) = session() {
        let _ = s.set_item(&format!("lattice:list-{kind}-restore"), "1");
    }
}

/// On the list page: keeps the spot up to date, and returns to it after "Back"
/// once `ready` (the rows are on screen).
pub fn remember(kind: &'static str, ready: impl Fn() -> bool + 'static) {
    let save = move || {
        // Leaving scrolls the next page to the top after the URL has changed;
        // ignore that so it doesn't overwrite the spot.
        if window().location().pathname().ok().as_deref() != Some(&format!("/{kind}")) {
            return;
        }
        let spot = Spot { url: here(), y: scroll_box().map_or(0, |b| b.scroll_top()) };
        if let (Some(s), Ok(raw)) = (session(), serde_json::to_string(&spot)) {
            let _ = s.set_item(&format!("lattice:list-{kind}"), &raw);
        }
    };
    // Filters and "Show more" change the URL without remounting: save on each.
    let search = use_location().search;
    Effect::new(move |_| {
        search.track();
        save();
    });
    Effect::new(move |restored: Option<bool>| {
        if restored == Some(true) || !ready() {
            return restored.unwrap_or(false);
        }
        let flag = format!("lattice:list-{kind}-restore");
        if let Some(s) = session() {
            if s.get_item(&flag).ok().flatten().is_some()
                && let Some(spot) = load(kind).filter(|spot| spot.url == here())
            {
                request_animation_frame(move || {
                    if let Some(b) = scroll_box() {
                        b.set_scroll_top(spot.y);
                    }
                });
            }
            let _ = s.remove_item(&flag);
        }
        true
    });
    // Scroll doesn't bubble; capture catches the box's, even after it's re-rendered.
    let on_scroll = Closure::<dyn Fn()>::new(save);
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_capture(true);
    opts.set_passive(true);
    let _ = document().add_event_listener_with_callback_and_add_event_listener_options(
        "scroll",
        on_scroll.as_ref().unchecked_ref(),
        &opts,
    );
    let on_scroll = StoredValue::new_local(on_scroll);
    on_cleanup(move || {
        on_scroll.with_value(|f| {
            let _ = document().remove_event_listener_with_callback_and_bool("scroll", f.as_ref().unchecked_ref(), true);
        });
    });
}

/// "← Products" / "← Hampers" on a detail page, back to the list as it was.
#[component]
pub fn BackToList(kind: &'static str) -> impl IntoView {
    view! {
        <A href=list_href(kind) scroll=false on:click=move |_| mark_restore(kind)
            attr:class="mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)">
            <span aria-hidden="true">"←"</span>
            {if kind == "products" { " Products" } else { " Hampers" }}
        </A>
    }
}
