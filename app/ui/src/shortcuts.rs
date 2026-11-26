//! The app's shortcuts, as bound in Settings → Key bindings (port of
//! `shortcuts.tsx`).
//!
//! Undo/redo steps (undo.rs): ticks and review marks (kit.rs Selection), and
//! each move to another screen, undone by going back and redone by going
//! forward. With nothing to undo, undo still goes back a screen.
//!
//! Typing in a box keeps every key for the box. Screens with their own use of
//! a key run first and win with preventDefault (the studio canvas's undo).
//! Before changing screen, screens with unsaved work may object (leave.rs).

use crate::kit::on_window;
use crate::leave::{LeaveTo, may_leave};
use crate::undo;
use lattice_core::keybindings::{self as kb, ACTIONS, Bindings};
use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use std::time::Duration;
use wasm_bindgen::JsCast;

const KEY: &str = "lattice:keybindings";
const NOT_TEXT: [&str; 9] = ["checkbox", "radio", "button", "submit", "reset", "range", "color", "file", "image"];

/// This device's bindings, shared by the shortcuts and Settings → Key bindings.
#[derive(Clone, Copy)]
pub struct KeyBindings(pub RwSignal<Bindings>);

fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

impl KeyBindings {
    pub fn provide() {
        let saved: Bindings = storage()
            .and_then(|s| s.get_item(KEY).ok().flatten())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        provide_context(Self(RwSignal::new(kb::merge(&saved))));
    }

    /// None goes back to the defaults.
    pub fn save(&self, bindings: Option<Bindings>) {
        if let Some(s) = storage() {
            let _ = match bindings.as_ref().and_then(|b| serde_json::to_string(b).ok()) {
                Some(raw) => s.set_item(KEY, &raw),
                None => s.remove_item(KEY),
            };
        }
        self.0.set(bindings.unwrap_or_else(kb::defaults));
    }
}

/// Focus is in a box that takes typing (a just-clicked checkbox doesn't count).
fn typing_in(target: Option<web_sys::EventTarget>) -> bool {
    let Some(el) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else { return false };
    match el.tag_name().as_str() {
        _ if el.is_content_editable() => true,
        "TEXTAREA" | "SELECT" => true,
        "INPUT" => !NOT_TEXT.contains(&el.unchecked_ref::<web_sys::HtmlInputElement>().type_().as_str()),
        _ => false,
    }
}

fn found(selector: &str) -> Option<web_sys::Element> {
    document().query_selector(selector).ok().flatten()
}

#[component]
pub fn Shortcuts() -> impl IntoView {
    let bindings = expect_context::<KeyBindings>().0;
    let navigate = StoredValue::new_local(use_navigate());

    let go = move |to: LeaveTo| {
        if !may_leave(&to) {
            return;
        }
        let history = window().history().ok();
        match to {
            LeaveTo::Path(path) => navigate.with_value(|nav| nav(&path, Default::default())),
            LeaveTo::Forward => {
                if let Some(h) = history {
                    let _ = h.forward();
                }
            }
            LeaveTo::Back => {
                if let Some(h) = history.filter(|h| h.length().unwrap_or(0) > 1) {
                    let _ = h.back();
                }
            }
        }
    };

    let run = move |id: &str| match id {
        "undo" => {
            if !undo::undo() {
                go(LeaveTo::Back);
            }
        }
        "redo" => {
            undo::redo();
        }
        "back" => go(LeaveTo::Back),
        "forward" => go(LeaveTo::Forward),
        "search" => {
            if let Some(el) = found("main input[type=search], main input[name=q], main input#q")
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        }
        other => {
            if let Some(href) = ACTIONS.iter().find(|a| a.id == other).and_then(|a| a.href) {
                go(LeaveTo::Path(href.to_string()));
            }
        }
    };

    // Settings → Key bindings waiting for a key or button: leave it alone.
    let capturing = || found("[data-capturing]").is_some();

    on_window(ev::keydown, move |e| {
        let combo = kb::combo_from_key(&e.key(), e.ctrl_key(), e.meta_key(), e.alt_key(), e.shift_key());
        let Some(id) = bindings.with_untracked(|b| kb::action_for(b, combo.as_deref()).map(|a| a.id)) else { return };
        if e.repeat() || capturing() || typing_in(e.target()) || found("dialog[open]").is_some() {
            return;
        }
        if id == "search" {
            e.prevent_default(); // not the webview's own find
        }
        // After every other keydown listener, whatever order they were added in.
        set_timeout(
            move || {
                if !e.default_prevented() {
                    run(id)
                }
            },
            Duration::ZERO,
        );
    });

    // Middle and side buttons. Their defaults (back/forward, autoscroll) are
    // stopped only when the button is bound.
    let mouse = move |e: web_sys::MouseEvent| {
        let Some(id) = bindings.with_untracked(|b| kb::action_for(b, kb::combo_from_mouse(e.button())).map(|a| a.id))
        else {
            return;
        };
        if capturing() {
            return;
        }
        e.prevent_default();
        if e.type_() == "mouseup" {
            run(id);
        }
    };
    on_window(ev::mousedown, mouse);
    on_window(ev::mouseup, mouse);
    on_window(ev::auxclick, mouse);

    // A move to another screen is an undo step, unless it was Back/Forward.
    // ponytail: the webview's own history moves aren't undo steps, so after
    // one a screen step can go back from a different page than it was
    // recorded on. Track positions in history.state if that bites.
    let popped = StoredValue::new(false);
    on_window(ev::popstate, move |_| popped.set_value(true));
    let pathname = use_location().pathname;
    Effect::new(move |prev: Option<String>| {
        let path = pathname.get();
        if prev.is_some_and(|p| p != path) {
            if popped.get_value() {
                popped.set_value(false);
            } else {
                undo::record(move || go(LeaveTo::Back), move || go(LeaveTo::Forward));
            }
        }
        path
    });
}
