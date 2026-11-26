//! Settings → Key bindings (port of `key-bindings.tsx`), like a game's controls
//! screen: click a slot, then press a key, a combination or a mouse button
//! (middle, back, forward). Esc cancels; Backspace clears. Kept on this device.

use crate::shortcuts::KeyBindings;
use lattice_core::keybindings::{self as kb, ACTIONS};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// The studio's fixed shortcuts, shown beside the bindable ones.
const STUDIO_SHORTCUTS: [(&str, &str); 15] = [
    ("Undo / Redo", "Ctrl+Z / Ctrl+Shift+Z"),
    ("Save", "Ctrl+S"),
    ("Select several", "Shift+click, or drag a box"),
    ("Select all", "Ctrl+A"),
    ("Copy / Paste / Duplicate", "Ctrl+C / Ctrl+V / Ctrl+D"),
    ("Delete", "Del"),
    ("Nudge (×10 with Shift)", "Arrow keys"),
    ("Forward / Backward", "Ctrl+] / Ctrl+["),
    ("To front / To back", "Ctrl+Shift+] / Ctrl+Shift+["),
    ("Edit text", "Enter or double-click"),
    ("Zoom in / out / fit", "Ctrl+= / Ctrl+- / Ctrl+0"),
    ("Zoom at cursor", "Ctrl+scroll"),
    ("Pan", "Scroll, or hold Space and drag"),
    ("Place freely (no snapping)", "Hold Alt while dragging"),
    ("Snap line (0° / 90° / 45°)", "Hold Shift to lock 45° increments"),
];

/// A window listener in the capture phase, so it runs before any other, for
/// the component's lifetime.
fn capture<E: JsCast + 'static>(event: &'static str, cb: impl Fn(E) + 'static) {
    let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| cb(e.unchecked_into()));
    let _ = window().add_event_listener_with_callback_and_bool(event, cb.as_ref().unchecked_ref(), true);
    let cb = StoredValue::new_local(cb);
    on_cleanup(move || {
        cb.with_value(|cb| {
            let _ = window().remove_event_listener_with_callback_and_bool(event, cb.as_ref().unchecked_ref(), true);
        });
    });
}

#[component]
pub fn KeyBindingsTab() -> impl IntoView {
    let keys = expect_context::<KeyBindings>();
    let waiting = RwSignal::new(None::<(&'static str, usize)>);
    let note = RwSignal::new(String::new());

    let put = move |combo: String| {
        let Some((id, slot)) = waiting.get_untracked() else { return };
        let (next, moved) = kb::assign(&keys.0.get_untracked(), id, slot, &combo);
        keys.save(Some(next));
        let from = moved.and_then(|m| ACTIONS.iter().find(|a| a.id == m));
        note.set(from.map_or_else(String::new, |a| {
            format!("{} was on “{}”; it's moved here.", kb::combo_label(&combo), a.label)
        }));
        waiting.set(None);
    };

    capture("keydown", move |e: web_sys::KeyboardEvent| {
        if waiting.get_untracked().is_none() {
            return;
        }
        e.prevent_default();
        e.stop_propagation();
        let plain = !e.ctrl_key() && !e.meta_key() && !e.alt_key() && !e.shift_key();
        let key = e.key();
        if key == "Escape" {
            waiting.set(None);
        } else if plain && (key == "Backspace" || key == "Delete") {
            put(String::new());
        } else if let Some(c) = kb::combo_from_key(&key, e.ctrl_key(), e.meta_key(), e.alt_key(), e.shift_key()) {
            put(c);
        }
    });
    for event in ["mousedown", "mouseup", "auxclick"] {
        capture(event, move |e: web_sys::MouseEvent| {
            if waiting.get_untracked().is_none() {
                return;
            }
            let Some(combo) = kb::combo_from_mouse(e.button()) else {
                // A normal click elsewhere cancels; on the waiting slot, its own click does.
                let on_slot = e
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .and_then(|el| el.closest("[data-capturing]").ok().flatten())
                    .is_some();
                if e.type_() == "mousedown" && !on_slot {
                    waiting.set(None);
                }
                return;
            };
            e.prevent_default();
            e.stop_propagation();
            if e.type_() == "mouseup" {
                put(combo.to_string());
            }
        });
    }

    let is_waiting = move |id: &str, slot: usize| waiting.with(|w| w.is_some_and(|(i, s)| i == id && s == slot));
    let mut groups: Vec<&str> = Vec::new();
    for a in &ACTIONS {
        if !groups.contains(&a.group) {
            groups.push(a.group);
        }
    }

    view! {
        <div class="grid gap-4 lg:grid-cols-[1fr_20rem]">
            <section class="card overflow-hidden">
                <div class="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3">
                    <p class="text-xs text-(--color-muted)">
                        "Click a slot, then press a key, a combination or a mouse button. " <kbd>"Esc"</kbd> " cancels, "
                        <kbd>"Backspace"</kbd> " clears. They work anywhere except while typing in a box, and are kept on this device."
                    </p>
                    <button type="button" class="btn-secondary" on:click=move |_| { keys.save(None); note.set("Back to the defaults.".into()); }>
                        "Reset all"
                    </button>
                </div>
                {move || (!note.get().is_empty()).then(|| view! { <p role="status" class="border-b border-line bg-paper px-4 py-2 text-xs">{note.get()}</p> })}
                <table class="w-full text-sm">
                    <thead>
                        <tr class="text-left text-[11px] uppercase tracking-wide text-(--color-muted)">
                            <th class="px-4 py-2 font-medium">"Action"</th>
                            <th class="px-2 py-2 font-medium">"Key"</th>
                            <th class="px-2 py-2 font-medium">"Alternate"</th>
                        </tr>
                    </thead>
                    {groups.into_iter().map(|group| view! {
                        <tbody>
                            <tr><th colspan="3" class="bg-paper px-4 py-1.5 text-left text-xs font-semibold">{group}</th></tr>
                            {ACTIONS.iter().filter(|a| a.group == group).map(|a| view! {
                                <tr class="border-t border-line">
                                    <td class="px-4 py-1.5">{a.label}</td>
                                    {[0usize, 1].into_iter().map(|slot| {
                                        let (id, label) = (a.id, a.label);
                                        let combo = move || keys.0.with(|b| b.get(id).and_then(|s| s.get(slot)).cloned().unwrap_or_default());
                                        view! {
                                            <td class="px-2 py-1.5">
                                                <button
                                                    type="button"
                                                    attr:data-capturing=move || is_waiting(id, slot).then_some("")
                                                    on:click=move |_| waiting.set(if is_waiting(id, slot) { None } else { Some((id, slot)) })
                                                    aria-label=move || format!("{label}, {}: {}. Click to change.", if slot == 1 { "alternate" } else { "key" }, if combo().is_empty() { "none".to_string() } else { kb::combo_label(&combo()).to_string() })
                                                    class=move || format!("min-w-36 rounded-md border px-2.5 py-1 text-left font-mono text-xs {}", if is_waiting(id, slot) {
                                                        "animate-pulse border-(--color-brand) bg-(--color-brand)/10 text-(--color-brand-dark)"
                                                    } else {
                                                        "border-line bg-white hover:border-(--color-brand)"
                                                    })
                                                >
                                                    {move || if is_waiting(id, slot) {
                                                        "Press a key or button…".to_string()
                                                    } else if combo().is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        kb::combo_label(&combo()).to_string()
                                                    }}
                                                </button>
                                            </td>
                                        }
                                    }).collect_view()}
                                </tr>
                            }).collect_view()}
                        </tbody>
                    }).collect_view()}
                </table>
            </section>
            <section class="card self-start p-4">
                <h2 class="text-sm font-semibold">"Hamper studio"</h2>
                <p class="mt-0.5 mb-3 text-xs text-(--color-muted)">"Fixed inside the canvas editor."</p>
                <dl class="space-y-1.5">
                    {STUDIO_SHORTCUTS.iter().map(|(what, keys)| view! {
                        <div class="flex justify-between gap-3 text-xs">
                            <dt class="text-(--color-muted)">{*what}</dt>
                            <dd class="text-right font-mono">{*keys}</dd>
                        </div>
                    }).collect_view()}
                </dl>
            </section>
        </div>
    }
}
