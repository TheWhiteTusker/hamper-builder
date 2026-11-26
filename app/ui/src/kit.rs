//! Small shared pieces, as in the Next app's `components/`: menus, image
//! preview, the ticked-rows selection, links into the product screens.

use lattice_core::images::{thumb_url, view_url};
use leptos::html;
use leptos::prelude::*;
use leptos_router::components::A;

/// A window listener that lives as long as the component adding it. Leptos
/// leaves `window_event_listener` attached until removed by hand, so without
/// this every visit to a screen added another, still running on dead state.
pub fn on_window<E>(event: E, cb: impl Fn(E::EventType) + 'static)
where
    E: leptos::ev::EventDescriptor + 'static,
    E::EventType: wasm_bindgen::JsCast,
{
    let handle = window_event_listener(event, cb);
    on_cleanup(move || handle.remove());
}

/// "19 Nov 2026, 3:42 pm" in India's time, as
/// `toLocaleString("en-IN", { timeZone: "Asia/Kolkata", dateStyle: "medium", timeStyle: "short" })`.
pub fn when(ts: &str) -> String {
    let d = js_sys::Date::new(&ts.into());
    let opts = js_sys::Object::new();
    for (k, v) in [("timeZone", "Asia/Kolkata"), ("dateStyle", "medium"), ("timeStyle", "short")] {
        let _ = js_sys::Reflect::set(&opts, &k.into(), &v.into());
    }
    d.to_locale_string("en-IN", &opts).as_string().unwrap_or_default()
}

/// The History tab beside Costing: that product's changes.
pub fn history_href(code: &str) -> String {
    format!("/cost-calculator/history/{}", encode_parts(code))
}

/// `/products/LC/0001/WL`, each part encoded (the code's slashes are path separators).
pub fn product_href(code: &str) -> String {
    format!("/products/{}", encode_parts(code))
}

/// Where a product's costing lives: `/cost-calculator/LC/0001/WL`.
pub fn costing_href(code: &str) -> String {
    format!("/cost-calculator/{}", encode_parts(code))
}

fn encode_parts(code: &str) -> String {
    code.split('/').map(|p| String::from(js_sys::encode_uri_component(p))).collect::<Vec<_>>().join("/")
}

/// A product code from a catch-all route's segments.
pub fn decode_code(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|p| js_sys::decode_uri_component(p).map(String::from).unwrap_or_else(|_| p.to_string()))
        .collect::<Vec<_>>()
        .join("/")
}

/// The browser's confirm dialog.
pub fn confirm(message: &str) -> bool {
    window().confirm_with_message(message).unwrap_or(false)
}

pub fn alert(message: &str) {
    let _ = window().alert_with_message(message);
}

pub struct MenuItem {
    pub href: &'static str,
    pub title: &'static str,
    pub hint: &'static str,
}

/// A button that opens a short list of links (native `<details>`).
#[component]
pub fn Menu(label: &'static str, items: Vec<MenuItem>) -> impl IntoView {
    let details = NodeRef::<html::Details>::new();
    let close = move || {
        if let Some(d) = details.get() {
            d.set_open(false);
        }
    };
    on_window(leptos::ev::pointerdown, move |e| {
        let inside = details
            .get()
            .zip(e.target())
            .is_some_and(|(d, t)| t.dyn_ref::<web_sys::Node>().is_some_and(|n| d.contains(Some(n))));
        if !inside {
            close();
        }
    });
    view! {
        <details node_ref=details class="relative">
            <summary class="btn-secondary cursor-pointer list-none [&::-webkit-details-marker]:hidden">
                {label}
                <span aria-hidden="true" class="text-xs">"▾"</span>
            </summary>
            <div class="absolute right-0 z-20 mt-1 w-64 rounded-lg border border-line bg-white p-1 shadow-lg">
                {items
                    .into_iter()
                    .map(|i| {
                        view! {
                            <A href=i.href attr:class="block rounded-md px-3 py-2 hover:bg-paper focus:bg-paper focus:outline-none">
                                <span class="block text-sm font-medium text-(--color-ink)">{i.title}</span>
                                <span class="block text-xs text-(--color-muted)">{i.hint}</span>
                            </A>
                        }
                    })
                    .collect_view()}
            </div>
        </details>
    }
}

use wasm_bindgen::JsCast;

/// An `<img>` of `src` that falls back to `fallback` if that copy is missing
/// (older photos have no WebP copies).
#[component]
pub fn FallbackImg(
    src: String,
    fallback: String,
    #[prop(into)] alt: String,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let on_error = move |e: leptos::ev::ErrorEvent| {
        if let Some(img) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlImageElement>().ok())
            && img.src() != fallback
        {
            img.set_src(&fallback);
        }
    };
    view! { <img src=src alt=alt class=class loading="lazy" on:error=on_error /> }
}

/// A thumbnail that opens the full photo in a modal (Esc or a backdrop click closes it).
#[component]
pub fn ImagePreview(src: String, #[prop(into)] alt: String, class: &'static str) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    let (thumb, view, original) = (thumb_url(&src), view_url(&src), src.clone());
    let backdrop = format!("background: url(\"{thumb}\") center / contain no-repeat");
    let preload = view.clone();
    let alt2 = alt.clone();
    view! {
        <button
            type="button"
            title=format!("Preview {alt}")
            class=format!("relative block cursor-zoom-in overflow-hidden {class}")
            on:click=move |_| {
                if let Some(d) = dialog.get() {
                    let _ = d.show_modal();
                }
            }
            on:pointerenter=move |_| {
                if let Ok(img) = web_sys::HtmlImageElement::new() {
                    img.set_src(&preload);
                }
            }
        >
            <FallbackImg src=thumb fallback=original.clone() alt=alt.clone() class="absolute inset-0 h-full w-full object-cover" />
        </button>
        <dialog
            node_ref=dialog
            class="m-auto max-h-[96vh] max-w-[96vw] rounded-xl bg-white p-2 shadow-2xl backdrop:bg-black/70"
            on:click=move |e| {
                let on_backdrop = dialog.get().zip(e.target()).is_some_and(|(d, t)| {
                    t.dyn_ref::<web_sys::Element>().is_some_and(|el| el == d.unchecked_ref::<web_sys::Element>())
                });
                if on_backdrop && let Some(d) = dialog.get() {
                    d.close();
                }
            }
        >
            <img src=view alt=alt2.clone() loading="lazy" style=backdrop class="block h-[86vh] w-[94vw] max-w-350 object-contain" />
            <div class="flex items-center justify-between gap-3 px-1 pt-2 text-xs">
                <span class="truncate text-(--color-muted)">{alt2}</span>
                <form method="dialog">
                    <button class="btn-secondary py-1 text-xs">"Close"</button>
                </form>
            </div>
        </dialog>
    }
}

/// Ticked rows of a list (products, hampers), and its rows marked for review,
/// kept on this device until unticked: they survive other screens, restarts
/// and filters. One store per list for the whole window, so every screen and
/// every undo step sees the same ticks (port of `selection.ts`).
#[derive(Clone, Copy)]
pub struct Selection {
    kind: &'static str,
}

thread_local! {
    static STORES: std::cell::RefCell<std::collections::HashMap<String, ArcRwSignal<Vec<String>>>> =
        std::cell::RefCell::default();
}

fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// `products`, `review-products`, …: the store behind one list's ticks or marks.
fn store(key: &str) -> ArcRwSignal<Vec<String>> {
    STORES.with(|s| {
        s.borrow_mut()
            .entry(key.to_string())
            .or_insert_with(|| {
                let saved = storage()
                    .and_then(|s| s.get_item(&format!("lattice:selected-{key}")).ok().flatten())
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default();
                ArcRwSignal::new(saved)
            })
            .clone()
    })
}

type Change = Vec<(String, Vec<String>)>;

fn apply(to: &Change) {
    for (key, ids) in to {
        if let (Some(s), Ok(raw)) = (storage(), serde_json::to_string(ids)) {
            let _ = s.set_item(&format!("lattice:selected-{key}"), &raw);
        }
        store(key).set(ids.clone());
    }
}

/// Every tick, mark and clear goes through here, so each is one undo step.
fn change(next: Change) {
    let prev: Change = next.iter().map(|(k, _)| (k.clone(), store(k).get_untracked())).collect();
    apply(&next);
    crate::undo::record(move || apply(&prev), move || apply(&next));
}

impl Selection {
    pub fn new(kind: &'static str) -> Self {
        Self { kind }
    }

    fn review(&self) -> String {
        format!("review-{}", self.kind)
    }

    fn holds(key: &str, id: &str) -> bool {
        store(key).with(|ids| ids.iter().any(|x| x == id))
    }

    /// A row is ticked or marked for review, never both: doing one undoes the other.
    fn toggle(own: &str, rival: &str, id: &str, on: bool) {
        let mut cur = store(own).get_untracked();
        cur.retain(|x| x != id);
        if on {
            cur.push(id.to_string());
        }
        let mut next = vec![(own.to_string(), cur)];
        if on && Self::holds(rival, id) {
            let mut r = store(rival).get_untracked();
            r.retain(|x| x != id);
            next.push((rival.to_string(), r));
        }
        change(next);
    }

    pub fn has(&self, id: &str) -> bool {
        Self::holds(self.kind, id)
    }

    pub fn set(&self, id: &str, on: bool) {
        Self::toggle(self.kind, &self.review(), id, on);
    }

    pub fn marked(&self, id: &str) -> bool {
        Self::holds(&self.review(), id)
    }

    pub fn mark(&self, id: &str, on: bool) {
        Self::toggle(&self.review(), self.kind, id, on);
    }

    pub fn clear(&self) {
        change(vec![(self.kind.to_string(), Vec::new())]);
    }

    pub fn len(&self) -> usize {
        store(self.kind).with(Vec::len)
    }

    /// "product" for "products".
    pub fn one(&self) -> &'static str {
        self.kind.strip_suffix('s').unwrap_or(self.kind)
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

/// A ⋮ button with a short list of actions.
#[component]
pub fn Dots(
    #[prop(into)] label: String,
    items: Vec<(Signal<String>, Callback<()>, Signal<bool>)>,
    class: &'static str,
) -> impl IntoView {
    let details = NodeRef::<html::Details>::new();
    on_window(leptos::ev::pointerdown, move |e| {
        let inside = details
            .get()
            .zip(e.target())
            .is_some_and(|(d, t)| t.dyn_ref::<web_sys::Node>().is_some_and(|n| d.contains(Some(n))));
        if !inside && let Some(d) = details.get() {
            d.set_open(false);
        }
    });
    view! {
        <details node_ref=details class="relative inline-block">
            <summary aria-label=label.clone() title=label class=format!("list-none [&::-webkit-details-marker]:hidden {class}")>"⋮"</summary>
            <div class="absolute right-0 z-30 mt-1 w-40 rounded-lg border border-line bg-white p-1 text-left shadow-lg">
                {items
                    .into_iter()
                    .map(|(text, run, disabled)| {
                        view! {
                            <button
                                type="button"
                                disabled=move || disabled.get()
                                on:click=move |_| {
                                    run.run(());
                                    if let Some(d) = details.get() {
                                        d.set_open(false);
                                    }
                                }
                                class="block w-full rounded-md px-3 py-2 text-left text-sm font-medium text-(--color-ink) hover:bg-paper focus:bg-paper focus:outline-none disabled:cursor-not-allowed disabled:opacity-45"
                            >
                                {text}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </details>
    }
}

/// ⋮ in the page header: Deselect all.
#[component]
pub fn ListMenu(sel: Selection) -> impl IntoView {
    let text = Signal::derive(move || match sel.len() {
        0 => "Deselect all".to_string(),
        n => format!("Deselect all ({n})"),
    });
    let clear = Callback::new(move |()| {
        let n = sel.len();
        let what =
            if n == 1 { format!("the 1 selected {}", sel.one()) } else { format!("all {n} selected {}", sel.kind()) };
        if confirm(&format!("Unselect {what}?")) {
            sel.clear();
        }
    });
    view! {
        <Dots label="More actions" items=vec![(text, clear, Signal::derive(move || sel.len() == 0))] class="btn-secondary cursor-pointer px-3 text-base leading-5" />
    }
}

/// "3 selected · Clear" above the list; hidden when nothing is selected.
#[component]
pub fn SelectionCount(sel: Selection) -> impl IntoView {
    view! {
        <Show when=move || sel.len() != 0>
            <div class="mb-3 flex items-center gap-3 text-sm">
                <span class="font-medium text-(--color-ink)">
                    {move || {
                        let n = sel.len();
                        format!("{n} {} selected", if n == 1 { sel.one() } else { sel.kind() })
                    }}
                </span>
                <button
                    type="button"
                    class="font-medium text-(--color-brand) hover:underline"
                    on:click=move |_| {
                        if confirm(&format!("Unselect all {}?", sel.kind())) {
                            sel.clear();
                        }
                    }
                >
                    "Clear"
                </button>
            </div>
        </Show>
    }
}

/// Tick box beside a row's number: ticks or unticks it. Shown on row hover or
/// keyboard focus (always on touch screens); the row's tint shows what's ticked.
#[component]
pub fn SelectBox(sel: Selection, id: String, #[prop(into)] label: String) -> impl IntoView {
    let id = StoredValue::new(id);
    let on = Memo::new(move |_| id.with_value(|id| sel.has(id)));
    view! {
        <input
            type="checkbox"
            prop:checked=on
            on:change=move |_| id.with_value(|id| sel.set(id, !on.get_untracked()))
            aria-label=format!("Select {label}")
            class="h-4 w-4 cursor-pointer accent-(--color-brand) opacity-0 group-hover:opacity-100 focus-visible:opacity-100 pointer-coarse:opacity-100"
        />
    }
}

/// Bookmark beside a row's tick box: marks it for review. Shown on row hover;
/// the row's red shows it's marked.
#[component]
pub fn ReviewMark(sel: Selection, id: String, #[prop(into)] label: String) -> impl IntoView {
    let id = StoredValue::new(id);
    let on = Memo::new(move |_| id.with_value(|id| sel.marked(id)));
    view! {
        <button
            type="button"
            on:click=move |_| id.with_value(|id| sel.mark(id, !on.get_untracked()))
            aria-pressed=move || on.get().to_string()
            aria-label=format!("Mark {label} for review")
            title=move || if on.get() { "Marked for review (click to unmark)" } else { "Mark for review" }
            class=move || format!(
                "cursor-pointer rounded p-0.5 opacity-0 group-hover:opacity-100 focus-visible:opacity-100 pointer-coarse:opacity-100 {}",
                if on.get() { "text-red-600" } else { "text-(--color-muted) hover:text-red-600" }
            )
        >
            <svg viewBox="0 0 24 24" class="h-4 w-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                fill=move || if on.get() { "currentColor" } else { "none" } aria-hidden="true">
                <path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z" />
            </svg>
        </button>
    }
}

/// A row's number, with its tick box and bookmark in the gutter left of the
/// table (`.row-gutter`). Ticked rows take the brand tint and marked ones red
/// (`row_tint`), so they show without hovering.
#[component]
pub fn RowNumber(sel: Selection, n: usize, id: String, #[prop(into)] label: String) -> impl IntoView {
    view! {
        <td class="td num relative w-12 text-(--color-muted)">
            {n}
            <div class="absolute top-1/2 right-full mr-1.5 flex -translate-y-1/2 items-center gap-1">
                <SelectBox sel=sel id=id.clone() label=label.clone() />
                <ReviewMark sel=sel id=id label=label />
            </div>
        </td>
    }
}

/// The row classes that show its tick or review mark. Never both
/// (Selection::toggle); the ! only covers the instant between the two writes.
pub const ROW_TINT: &str = "group has-checked:bg-(--color-brand)/15 has-aria-pressed:bg-red-100!";

/// An error box for a failed load or save.
#[component]
pub fn ErrorBox(#[prop(into)] message: Signal<Option<String>>) -> impl IntoView {
    view! {
        {move || message.get().map(|m| view! {
            <div role="alert" class="card mb-4 border-red-200 bg-red-50 p-3 text-sm text-red-800">{m}</div>
        })}
    }
}

/// Searchable dropdown (port of the builder's `Combo`): focus selects the
/// text and lists everything; typing narrows it, this list's matches first,
/// then matches from `all`; arrows, Enter, Tab and Escape work; × clears.
/// The list is `position: fixed`, so a scrolling table never clips it.
#[component]
pub fn Combo(
    label: &'static str,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] options: Signal<Vec<String>>,
    #[prop(optional, into)] all: Option<Signal<Vec<String>>>,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional, into)] disabled: Signal<bool>,
    on_pick: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    let open = RwSignal::new(false);
    let typing = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let highlight = RwSignal::new(-1i32);
    let pos = RwSignal::new((0.0, 0.0, 0.0, false));

    let filtered = Memo::new(move |_| {
        let opts = options.get();
        let q = search.get().trim().to_lowercase();
        if !typing.get() || q.is_empty() {
            return opts;
        }
        let source = all.map(|a| a.get()).filter(|a| !a.is_empty()).unwrap_or_else(|| opts.clone());
        let (mut here, mut other): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        for o in source.into_iter().filter(|o| o.to_lowercase().contains(&q)) {
            if opts.contains(&o) {
                here.push(o);
            } else {
                other.push(o);
            }
        }
        here.extend(other);
        here
    });
    let place = move || {
        if let Some(el) = input.get_untracked() {
            let r = el.get_bounding_client_rect();
            let win_h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(800.0);
            let win_w = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1200.0);
            let above = win_h - r.bottom() < 250.0 && r.top() > 250.0;
            let width = r.width().max(280.0);
            let left = r.left().min(win_w - width - 8.0).max(8.0);
            pos.set((if above { r.top() - 4.0 } else { r.bottom() + 4.0 }, left, width, above));
        }
    };
    let close = move || {
        open.set(false);
        typing.set(false);
        search.set(value.get_untracked());
    };
    let pick = move |opt: String| {
        let chosen = if opt == "All categories" { String::new() } else { opt };
        search.set(chosen.clone());
        typing.set(false);
        open.set(false);
        on_pick(chosen);
    };
    Effect::new(move |_| {
        if !open.get() {
            return highlight.set(-1);
        }
        let list = filtered.get();
        let v = value.get_untracked();
        let idx = if !typing.get() && !v.is_empty() { list.iter().position(|o| *o == v).unwrap_or(0) } else { 0 };
        highlight.set(if list.is_empty() { -1 } else { i32::try_from(idx).unwrap_or(0) });
    });
    on_window(leptos::ev::scroll, move |_| {
        if open.get_untracked() {
            place();
        }
    });
    on_window(leptos::ev::resize, move |_| {
        if open.get_untracked() {
            place();
        }
    });
    let root = NodeRef::<html::Div>::new();
    on_window(leptos::ev::mousedown, move |e| {
        if !open.get_untracked() {
            return;
        }
        let inside = root
            .get()
            .zip(e.target())
            .is_some_and(|(d, t)| t.dyn_ref::<web_sys::Node>().is_some_and(|n| d.contains(Some(n))));
        if !inside {
            close();
        }
    });
    let shown = move || if open.get() && typing.get() { search.get() } else { value.get() };
    let nth = move |h: i32| filtered.with_untracked(|f| usize::try_from(h).ok().and_then(|i| f.get(i).cloned()));
    view! {
        <div node_ref=root class="relative w-full min-w-0">
            <input
                node_ref=input
                aria-label=label
                placeholder=placeholder
                disabled=move || disabled.get()
                class=move || format!("input truncate pr-12 text-xs sm:text-sm {}", if disabled.get() { "cursor-not-allowed" } else { "cursor-pointer focus:cursor-text" })
                prop:value=shown
                on:focus=move |e| {
                    if let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                        el.select();
                    }
                    typing.set(false);
                    search.set(value.get_untracked());
                    open.set(true);
                    place();
                }
                on:input=move |e| {
                    let v = event_target_value(&e);
                    typing.set(true);
                    search.set(v.clone());
                    open.set(true);
                    if options.get_untracked().contains(&v) || v.is_empty() {
                        on_pick(v);
                    }
                }
                on:keydown=move |e| {
                    let n = i32::try_from(filtered.with_untracked(Vec::len)).unwrap_or(0);
                    let h = highlight.get_untracked();
                    match e.key().as_str() {
                        "ArrowDown" | "ArrowUp" => {
                            e.prevent_default();
                            if !open.get_untracked() {
                                open.set(true);
                                typing.set(false);
                                place();
                            } else if n > 0 {
                                highlight.set(if e.key() == "ArrowDown" { (h + 1) % n } else if h <= 0 { n - 1 } else { h - 1 });
                            }
                        }
                        "Enter" => {
                            if open.get_untracked() && h >= 0 && h < n && let Some(o) = nth(h) {
                                e.prevent_default();
                                pick(o);
                            }
                        }
                        "Escape" => {
                            e.prevent_default();
                            close();
                        }
                        "Tab" if open.get_untracked() => match nth(h).filter(|_| typing.get_untracked()) {
                            Some(o) => pick(o),
                            None => close(),
                        },
                        _ => {}
                    }
                }
            />
            <div class="absolute top-1/2 right-1 flex -translate-y-1/2 items-center gap-0.5">
                <Show when=move || (!value.get().is_empty() || (open.get() && !search.get().is_empty())) && !disabled.get()>
                    <button
                        type="button"
                        title=format!("Clear {label}")
                        aria-label=format!("Clear {label}")
                        class="rounded p-1 text-(--color-muted) transition-colors hover:bg-red-50 hover:text-red-700"
                        on:click=move |e| {
                            e.prevent_default();
                            e.stop_propagation();
                            search.set(String::new());
                            typing.set(false);
                            on_pick(String::new());
                            open.set(true);
                            if let Some(el) = input.get_untracked() {
                                let _ = el.focus();
                            }
                            place();
                        }
                    >
                        <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                            <path d="M18 6 6 18M6 6l12 12" />
                        </svg>
                    </button>
                </Show>
                <button
                    type="button"
                    tabindex="-1"
                    title=format!("Show {label} options")
                    aria-label=format!("Show {label} options")
                    disabled=move || disabled.get()
                    class="rounded p-1 text-(--color-muted) transition-colors hover:bg-slate-100 hover:text-(--color-ink)"
                    on:click=move |e| {
                        e.prevent_default();
                        e.stop_propagation();
                        if open.get_untracked() {
                            close();
                        } else {
                            open.set(true);
                            typing.set(false);
                            search.set(value.get_untracked());
                            if let Some(el) = input.get_untracked() {
                                let _ = el.focus();
                                el.select();
                            }
                            place();
                        }
                    }
                >
                    <svg
                        class=move || format!("h-3.5 w-3.5 transition-transform duration-150 {}", if open.get() { "rotate-180" } else { "" })
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    >
                        <path d="m6 9 6 6 6-6" />
                    </svg>
                </button>
            </div>
            <Show when=move || open.get()>
                <div
                    class="max-h-60 overflow-y-auto rounded-lg border border-(--color-line) bg-white py-1 text-xs shadow-xl sm:text-sm"
                    style=move || {
                        let (top, left, width, above) = pos.get();
                        format!(
                            "position: fixed; top: {top}px; left: {left}px; width: {width}px; z-index: 9999; transform: {}",
                            if above { "translateY(-100%)" } else { "none" }
                        )
                    }
                >
                    {move || {
                        let list = filtered.get();
                        if list.is_empty() {
                            return view! { <div class="px-3 py-3 text-center text-xs text-(--color-muted)">"No matching options"</div> }.into_any();
                        }
                        let v = value.get();
                        list.into_iter()
                            .enumerate()
                            .map(|(i, opt)| {
                                let selected = opt == v || (v.is_empty() && opt == "All categories");
                                let idx = i32::try_from(i).unwrap_or(0);
                                let chosen = opt.clone();
                                let body = match opt.rsplit_once(" — ") {
                                    Some((name, code)) if !name.is_empty() => {
                                        let (name, code) = (name.to_string(), code.to_string());
                                        view! {
                                            <div class="flex w-full min-w-0 items-center justify-between pr-1">
                                                <span class="truncate">{name}</span>
                                                <span class="ml-2 shrink-0 font-mono text-[11px] font-normal text-(--color-muted)">{code}</span>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    _ => view! { <span class="truncate pr-1">{opt.clone()}</span> }.into_any(),
                                };
                                view! {
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "flex w-full cursor-pointer select-none items-center justify-between px-3 py-1.5 text-left transition-colors {}",
                                            if highlight.get() == idx { "bg-paper text-(--color-ink)" } else if selected { "bg-slate-50 font-semibold text-(--color-ink)" } else { "text-(--color-ink)" }
                                        )
                                        on:mousedown=|e| e.prevent_default()
                                        on:click=move |_| pick(chosen.clone())
                                        on:mouseenter=move |_| highlight.set(idx)
                                    >
                                        {body}
                                        {selected.then(|| view! {
                                            <svg class="ml-1 h-3.5 w-3.5 shrink-0 text-(--color-brand)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                                                <path d="M20 6 9 17l-5-5" />
                                            </svg>
                                        })}
                                    </button>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
            </Show>
        </div>
    }
}

/// `String(n)` of a JS number (no "-0"): how the old app showed a number in an input.
pub fn js_str(x: f64) -> String {
    if x == 0.0 { "0".into() } else { x.to_string() }
}

pub fn dstr(d: lattice_core::Decimal) -> String {
    js_str(lattice_core::js::f(d))
}

/// A whole percent from a stored fraction: `String(round2(v * 100))`.
pub fn pct_str(d: lattice_core::Decimal) -> String {
    js_str(lattice_core::js::round2(lattice_core::js::f(d) * 100.0))
}

/// A JS number in a request body, as `JSON.stringify` wrote it.
pub fn json_num(x: f64) -> serde_json::Value {
    serde_json::Number::from_f64(x).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// GSTIN with "Fetch details": fills the name and billing address from the GST
/// register, through the Worker (which holds the API key).
#[component]
pub fn GstinField(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    can_edit: bool,
    on_found: impl Fn(String, String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let fetching = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let fetch = move || {
        let gstin = value.get_untracked().trim().to_uppercase();
        if gstin.chars().count() != 15 {
            return;
        }
        fetching.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let args = serde_json::json!({ "gstin": gstin });
            match crate::tauri::invoke::<_, lattice_core::gstin::GstDetails>("gstin_lookup", &args).await {
                Ok(d) => on_found(d.name, d.billing_address),
                Err(e) => error.set(e.to_string()),
            }
            fetching.set(false);
        });
    };
    view! {
        <div>
            <label class="label" for=id>{label}</label>
            <div class="mt-1 flex gap-2">
                <input id=id class="input font-mono uppercase" maxlength="15" placeholder="29AAECB4326R1Z5" disabled=!can_edit prop:value=value
                    on:input=move |e| {
                        value.set(event_target_value(&e).to_uppercase());
                        error.set(String::new());
                    } />
                {can_edit.then(|| view! {
                    <button type="button" class="btn-secondary whitespace-nowrap px-3 text-xs"
                        disabled=move || fetching.get() || value.with(|v| v.trim().chars().count() != 15) on:click=move |_| fetch()>
                        {move || if fetching.get() { "Fetching…" } else { "Fetch details" }}
                    </button>
                })}
            </div>
            {move || (!error.get().is_empty()).then(|| view! { <p class="mt-1 text-xs text-amber-700">{error.get()}</p> })}
        </div>
    }
}
