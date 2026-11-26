//! Escape asks to close the app. It runs full screen with no title bar, so this
//! pop-up (shown on the landing page and the dashboard) is the way out.

use crate::kit::on_window;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn ExitOnEscape() -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    on_window(leptos::ev::keydown, move |e| {
        if e.key() != "Escape" || e.repeat() || e.default_prevented() {
            return;
        }
        let Some(d) = dialog.get_untracked() else { return };
        // Open already: the dialog's own Escape handling is the Cancel.
        if d.open() {
            return;
        }
        // Another pop-up, or a field where Escape means something, comes first.
        let doc = document();
        if doc.query_selector("dialog[open], [role=dialog]").ok().flatten().is_some() {
            return;
        }
        if doc.active_element().is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")) {
            return;
        }
        e.prevent_default();
        let _ = d.show_modal();
    });
    let quit = move |_| {
        spawn_local(async {
            let _ = crate::tauri::invoke::<_, ()>("quit_app", &serde_json::json!({})).await;
        })
    };

    view! {
        <dialog
            node_ref=dialog
            aria-labelledby="exit-title"
            class="m-auto w-80 rounded-xl border border-line bg-white p-5 text-(--color-ink) shadow-xl backdrop:bg-black/40"
        >
            <h2 id="exit-title" class="text-base font-semibold">"Exit Lattice Lane?"</h2>
            <p class="mt-1 text-sm text-(--color-muted)">"The app will close."</p>
            <form method="dialog" class="mt-5 flex justify-end gap-2">
                <button type="submit" class="btn-secondary">"Cancel"</button>
                <button type="button" class="btn-primary" autofocus on:click=quit>"Exit"</button>
            </form>
        </dialog>
    }
}
