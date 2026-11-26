//! The studio's panels and bars (ports of `side-panels.tsx`, `panels/*`,
//! `context-toolbar.tsx`, `toolbar/*`, `layers-panel.tsx`, `stage/stage-*`).

use super::editor::{DRAG_MIME, Ed, Tool};
use super::{PickerProduct, Thumb};
use lattice_core::canvas::arrange::{LayerMove, PageAlign};
use lattice_core::canvas::{
    Align, Decoration, ELEMENTS, FONTS, Fill, Fit, FontStyle, LineCap, PRESETS, Shape, TEXT_COMBOS, TEXT_PRESETS,
    document_colors, element, fill_css, layer_label, make_text,
};
use lattice_core::images::thumb_url;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Products,
    Uploads,
    Text,
    Elements,
    Background,
}

const RAIL: [(Tab, &str, &str); 5] = [
    (Tab::Products, "Products", "M3 7l9-4 9 4-9 4-9-4zm0 0v10l9 4 9-4V7"),
    (Tab::Uploads, "Uploads", "M12 16V4m0 0l-4 4m4-4l4 4M4 16v4h16v-4"),
    (Tab::Text, "Text", "M5 6V4h14v2M12 4v16m-3 0h6"),
    (Tab::Elements, "Elements", "M4 4h7v7H4zM17.5 4a3.5 3.5 0 110 7 3.5 3.5 0 010-7zM4 20l3.5-6 3.5 6zM14 14h6v6h-6z"),
    (
        Tab::Background,
        "Background",
        "M12 3a9 9 0 100 18c1 0 1.5-.8 1.5-1.5 0-.4-.2-.8-.4-1.1-.3-.3-.4-.7-.4-1.1 0-.8.7-1.5 1.5-1.5H16a5 5 0 005-5c0-4.4-4-8-9-8z",
    ),
];

fn icon(d: &'static str) -> impl IntoView {
    view! {
        <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d=d /></svg>
    }
}

const TOOL_BTN: &str = "inline-flex h-8 min-w-8 items-center justify-center gap-1 rounded-md px-2 text-[12.5px] text-(--st-text) hover:bg-(--st-hover) disabled:opacity-40";

#[component]
fn ToolButton(
    #[prop(into)] title: String,
    #[prop(optional, into)] active: Signal<bool>,
    on_click: impl Fn() + 'static,
    children: Children,
) -> impl IntoView {
    view! {
        <button type="button" title=title.clone() aria-label=title
            class=move || format!("{TOOL_BTN} {}", if active.get() { "bg-(--st-accent-soft) text-(--st-accent)" } else { "" })
            on:click=move |_| on_click()>{children()}</button>
    }
}

#[component]
pub fn Rail(ed: Ed) -> impl IntoView {
    view! {
        <nav class="flex w-[72px] shrink-0 flex-col items-center gap-1 border-r border-(--st-line) bg-(--st-panel) py-2">
            {RAIL.map(|(tab, label, d)| view! {
                <button type="button" on:click=move |_| ed.tab.update(|t| *t = if *t == Some(tab) { None } else { Some(tab) })
                    class=move || format!("flex w-[62px] flex-col items-center gap-1 rounded-lg py-2 text-[10.5px] transition-colors {}",
                        if ed.tab.get() == Some(tab) { "bg-(--st-panel-2) text-(--st-text)" } else { "text-(--st-muted) hover:text-(--st-text)" })>
                    <span class=move || if ed.tab.get() == Some(tab) { "text-(--st-accent)" } else { "" }>{icon(d)}</span>
                    {label}
                </button>
            }).collect_view()}
        </nav>
    }
}

#[component]
pub fn SidePanel(ed: Ed) -> impl IntoView {
    let label = move || RAIL.iter().find(|(t, _, _)| Some(*t) == ed.tab.get()).map(|r| r.1).unwrap_or_default();
    view! {
        <aside class="flex w-[300px] shrink-0 flex-col border-r border-(--st-line) bg-(--st-panel)">
            <div class="flex h-11 items-center justify-between px-4">
                <span class="text-[14px] font-semibold">{label}</span>
                <button type="button" class=TOOL_BTN aria-label="Close panel" title="Close panel" on:click=move |_| ed.tab.set(None)>"×"</button>
            </div>
            <div class="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
                {move || match ed.tab.get() {
                    Some(Tab::Products) => view! { <ProductsPanel ed=ed /> }.into_any(),
                    Some(Tab::Uploads) => view! { <UploadsPanel ed=ed /> }.into_any(),
                    Some(Tab::Text) => view! { <TextPanel ed=ed /> }.into_any(),
                    Some(Tab::Elements) => view! { <ElementsPanel ed=ed /> }.into_any(),
                    Some(Tab::Background) => view! { <BackgroundPanel ed=ed /> }.into_any(),
                    None => ().into_any(),
                }}
            </div>
        </aside>
    }
}

fn drag_data(e: &web_sys::DragEvent, product: Option<&PickerProduct>, url: &str, name: &str) {
    if let Some(dt) = e.data_transfer() {
        let product = product
            .map(|p| serde_json::json!({ "id": p.id, "code": p.code, "name": p.name, "image_url": p.image_url }));
        let _ =
            dt.set_data(DRAG_MIME, &serde_json::json!({ "product": product, "url": url, "name": name }).to_string());
        dt.set_effect_allowed("copy");
    }
}

#[component]
fn ProductsPanel(ed: Ed) -> impl IntoView {
    let q = RwSignal::new(String::new());
    let (products, ours) = ed.config.with_value(|c| (c.products.clone(), c.hamper_product_ids.clone()));
    let products = StoredValue::new(products);
    let ours = StoredValue::new(ours);
    let tile = move |p: PickerProduct| {
        let url = p.image_url.clone();
        let draggable = url.is_some().then_some("true");
        view! {
            <button type="button" draggable=draggable title=format!("{} · {}", p.name, p.code)
                class="group flex flex-col gap-1 rounded-lg p-1 text-left hover:bg-(--st-hover) disabled:opacity-40" disabled=draggable.is_none()
                on:dragstart={ let p = p.clone(); move |e| if let Some(u) = &p.image_url { drag_data(&e, Some(&p), u, &p.name) } }
                on:click={ let p = p.clone(); move |_| if let Some(u) = p.image_url.clone() { ed.place_image(p.clone(), u, None) } }>
                <span class="checkerboard flex aspect-square w-full items-center justify-center overflow-hidden rounded-md border border-(--st-line)">
                    {match url {
                        Some(u) => view! { <img src=thumb_url(&u) alt="" loading="lazy" class="h-full w-full object-contain" /> }.into_any(),
                        None => view! { <span class="text-[10px] text-(--st-muted)">"No photo"</span> }.into_any(),
                    }}
                </span>
                <span class="truncate text-[11px]">{p.name.clone()}</span>
            </button>
        }
    };
    let filtered = move |mine: bool| {
        let s = q.get().trim().to_lowercase();
        products.with_value(|ps| {
            ps.iter()
                .filter(|p| ours.with_value(|o| o.contains(&p.id)) == mine)
                .filter(|p| s.is_empty() || p.name.to_lowercase().contains(&s) || p.code.to_lowercase().contains(&s))
                .take(if mine { usize::MAX } else { 120 })
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    view! {
        <input type="search" class="mb-3 h-9 w-full rounded-md border border-(--st-line) bg-(--st-panel-2) px-3 text-[13px] outline-none focus:border-(--st-accent)"
            placeholder="Search products" prop:value=q on:input=move |e| q.set(event_target_value(&e)) />
        {move || {
            let mine = filtered(true);
            (!mine.is_empty()).then(|| view! {
                <p class="mb-1 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"In this hamper"</p>
                <div class="mb-3 grid grid-cols-3 gap-1">{mine.into_iter().map(tile).collect_view()}</div>
            })
        }}
        <p class="mb-1 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"All products"</p>
        <div class="grid grid-cols-3 gap-1">{move || filtered(false).into_iter().map(tile).collect_view()}</div>
        <p class="mt-3 text-[11px] text-(--st-muted)">"Click or drag a product onto the page."</p>
    }
}

fn input_files(e: &leptos::ev::Event) -> Vec<web_sys::File> {
    let input = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
    let files = input
        .as_ref()
        .and_then(|i| i.files())
        .map(|l| (0..l.length()).filter_map(|i| l.get(i)).collect())
        .unwrap_or_default();
    if let Some(i) = input {
        i.set_value("");
    }
    files
}

#[component]
fn UploadsPanel(ed: Ed) -> impl IntoView {
    view! {
        <label class="flex cursor-pointer items-center justify-center gap-2 rounded-lg bg-(--st-accent) px-3 py-2.5 text-[13px] font-medium text-(--st-on-accent) hover:opacity-90">
            {move || if ed.uploading.get() { "Uploading…" } else { "Upload images" }}
            <input type="file" accept="image/*" multiple=true class="sr-only" on:change=move |e| ed.upload_images(input_files(&e)) />
        </label>
        <button type="button" class="mt-2 w-full rounded-lg border border-(--st-line) px-3 py-2 text-[13px] hover:bg-(--st-hover)" on:click=move |_| ed.add_logo()>
            "Add the Lattice Lane logo"
        </button>
        <div class="mt-3 grid grid-cols-2 gap-2">
            {move || ed.uploads.get().into_iter().map(|(url, name)| {
                let (u2, n2) = (url.clone(), name.clone());
                let (u3, n3) = (url.clone(), name.clone());
                view! {
                    <button type="button" draggable="true" title=name.clone() class="checkerboard aspect-square overflow-hidden rounded-md border border-(--st-line)"
                        on:dragstart=move |e| drag_data(&e, None, &u3, &n3)
                        on:click=move |_| ed.add_image(u2.clone(), n2.clone(), None)>
                        <img src=url alt="" class="h-full w-full object-contain" />
                    </button>
                }
            }).collect_view()}
        </div>
        <Show when=move || ed.uploads.with(Vec::is_empty)>
            <p class="mt-3 text-[12px] text-(--st-muted)">"Images you upload here appear in this list and on the page."</p>
        </Show>
    }
}

#[component]
fn TextPanel(ed: Ed) -> impl IntoView {
    let add = move |text: &'static str, s: lattice_core::canvas::TextStyle| {
        let l = ed.with_canvas(|c| make_text(c, super::uuid(), text, &s));
        ed.add(l);
    };
    view! {
        <div class="space-y-2">
            {TEXT_PRESETS.map(|(label, s, preview)| view! {
                <button type="button" class="w-full rounded-lg border border-(--st-line) px-3 py-2.5 text-left hover:bg-(--st-hover)"
                    style=format!("font-family: \"{}\"; font-size: {preview}px; font-weight: {}", s.font_family, if s.font_style.bold() { 700 } else { 400 })
                    on:click=move |_| add(label, s)>{label}</button>
            }).collect_view()}
        </div>
        <p class="mt-4 mb-2 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"Combinations"</p>
        <div class="grid grid-cols-2 gap-2">
            {TEXT_COMBOS.map(|(text, s)| view! {
                <button type="button" class="flex h-20 items-center justify-center overflow-hidden rounded-lg border border-(--st-line) bg-white px-2 text-center hover:border-(--st-accent)"
                    style=format!("font-family: \"{}\"; color: {}; font-size: {}px; font-weight: {}; font-style: {}; letter-spacing: {}px",
                        s.font_family, s.color, (s.font_size / 5.0).min(26.0), if s.font_style.bold() { 700 } else { 400 }, if s.font_style.italic() { "italic" } else { "normal" }, s.letter_spacing / 4.0)
                    on:click=move |_| add(text, s)>{text}</button>
            }).collect_view()}
        </div>
    }
}

#[component]
fn ElementsPanel(ed: Ed) -> impl IntoView {
    view! {
        <p class="mb-2 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"Draw"</p>
        <div class="mb-4 flex gap-2">
            <button type="button" class=move || format!("flex-1 rounded-lg border px-3 py-2 text-[12.5px] {}", if ed.tool.get() == Tool::Line { "border-(--st-accent) bg-(--st-accent-soft)" } else { "border-(--st-line) hover:bg-(--st-hover)" })
                on:click=move |_| ed.tool.update(|t| *t = if *t == Tool::Line { Tool::Select } else { Tool::Line })>"Line"</button>
            <button type="button" class=move || format!("flex-1 rounded-lg border px-3 py-2 text-[12.5px] {}", if ed.tool.get() == Tool::Curve { "border-(--st-accent) bg-(--st-accent-soft)" } else { "border-(--st-line) hover:bg-(--st-hover)" })
                on:click=move |_| ed.tool.update(|t| *t = if *t == Tool::Curve { Tool::Select } else { Tool::Curve })>"Curve"</button>
        </div>
        <p class="mb-2 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"Shapes and lines"</p>
        <div class="grid grid-cols-4 gap-2">
            {ELEMENTS.map(|(label, svg)| view! {
                <button type="button" title=label aria-label=label class="flex aspect-square items-center justify-center rounded-lg border border-(--st-line) text-[#c8a97e] hover:border-(--st-accent)"
                    on:click=move |_| {
                        if let Some(l) = ed.with_canvas(|c| element(label, c, super::uuid())) {
                            ed.add(l);
                        }
                    }>
                    <svg class="h-8 w-8" viewBox="0 0 48 48" fill="currentColor" inner_html=svg></svg>
                </button>
            }).collect_view()}
        </div>
    }
}

const PALETTE: [&str; 12] = [
    "#ffffff", "#faf8ee", "#f1ebe0", "#e7e1d0", "#ddcf8b", "#b5c2b1", "#54655b", "#2c332f", "#1f1f1f", "#b08d57",
    "#c0392b", "#ff66c4",
];

#[component]
fn Swatches(ed: Ed, on_pick: impl Fn(String) + Copy + Send + Sync + 'static) -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-1.5">
            {move || {
                let mut colors = ed.with_canvas(document_colors);
                for p in PALETTE {
                    if !colors.iter().any(|c| c.eq_ignore_ascii_case(p)) {
                        colors.push(p.to_string());
                    }
                }
                colors.into_iter().map(|c| view! {
                    <button type="button" title=c.clone() class="h-7 w-7 rounded-full border border-(--st-line) shadow-xs" style=format!("background: {c}")
                        on:click={ let c = c.clone(); move |_| on_pick(c.clone()) }></button>
                }).collect_view()
            }}
        </div>
    }
}

#[component]
fn BackgroundPanel(ed: Ed) -> impl IntoView {
    let fill = move || ed.with_canvas(|c| c.background.fill.clone());
    let set_fill = move |f: Fill| ed.change(|c| c.background.fill = f, Some("bg-fill"));
    let gradient = move || matches!(fill(), Fill::Linear { .. });
    view! {
        <p class="mb-2 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"Colour"</p>
        <Swatches ed=ed on_pick=move |c| set_fill(Fill::Solid { color: c }) />
        <div class="mt-3 flex items-center gap-2">
            <input type="color" class="h-8 w-12 cursor-pointer rounded border border-(--st-line)" prop:value=move || fill().main_color().to_string()
                on:input=move |e| {
                    let v = event_target_value(&e);
                    set_fill(match fill() { Fill::Linear { to, angle, .. } => Fill::Linear { from: v, to, angle }, Fill::Solid { .. } => Fill::Solid { color: v } });
                } />
            <label class="flex items-center gap-1.5 text-[12.5px]">
                <input type="checkbox" prop:checked=gradient on:change=move |e| {
                    let on = event_target_checked(&e);
                    let c = fill().main_color().to_string();
                    set_fill(if on { Fill::Linear { from: c, to: "#ffffff".into(), angle: 90.0 } } else { Fill::Solid { color: c } });
                } />
                "Gradient"
            </label>
        </div>
        {move || match fill() {
            Fill::Linear { from, to, angle } => view! {
                <div class="mt-2 flex items-center gap-2 text-[12px]">
                    "to"
                    <input type="color" class="h-8 w-12 rounded border border-(--st-line)" prop:value=to.clone()
                        on:input={ let from = from.clone(); move |e| set_fill(Fill::Linear { from: from.clone(), to: event_target_value(&e), angle }) } />
                    "angle"
                    <input type="range" min="0" max="360" class="flex-1" prop:value=angle.to_string()
                        on:input={ let (from, to) = (from.clone(), to.clone()); move |e| set_fill(Fill::Linear { from: from.clone(), to: to.clone(), angle: event_target_value(&e).parse().unwrap_or(angle) }) } />
                </div>
            }.into_any(),
            Fill::Solid { .. } => ().into_any(),
        }}
        <p class="mt-5 mb-2 text-[11px] font-semibold tracking-wide text-(--st-muted) uppercase">"Background image"</p>
        <label class="flex cursor-pointer items-center justify-center rounded-lg border border-(--st-line) px-3 py-2 text-[13px] hover:bg-(--st-hover)">
            {move || if ed.uploading.get() { "Uploading…" } else { "Upload a background image" }}
            <input type="file" accept="image/*" class="sr-only" on:change=move |e| if let Some(f) = input_files(&e).into_iter().next() { ed.upload_background(f) } />
        </label>
        <Show when=move || ed.with_canvas(|c| c.background.image_url.is_some())>
            <button type="button" class="mt-2 w-full rounded-lg px-3 py-2 text-[12.5px] text-red-700 hover:bg-red-50" on:click=move |_| ed.change(|c| c.background.image_url = None, None)>
                "Remove background image"
            </button>
        </Show>
    }
}

// ------------------------------------------------------------- toolbars

fn num_input(
    value: f64,
    step: &'static str,
    width: &'static str,
    label: &'static str,
    on: impl Fn(f64) + 'static,
) -> impl IntoView {
    view! {
        <input type="number" step=step aria-label=label title=label prop:value=format!("{}", (value * 100.0).round() / 100.0)
            class=format!("h-8 {width} rounded-md border border-(--st-line) bg-(--st-panel-2) px-2 text-[12.5px] tabular-nums outline-none focus:border-(--st-accent)")
            on:input={move |e| if let Ok(v) = event_target_value(&e).parse::<f64>() { on(v) }} />
    }
}

fn color_input(value: String, label: &'static str, on: impl Fn(String) + 'static) -> impl IntoView {
    view! {
        <input type="color" title=label aria-label=label prop:value=value class="h-8 w-9 cursor-pointer rounded border border-(--st-line) bg-transparent p-0.5"
            on:input=move |e| on(event_target_value(&e)) />
    }
}

#[component]
pub fn ContextToolbar(ed: Ed) -> impl IntoView {
    view! {
        <div class="flex min-h-12 flex-wrap items-center gap-1 border-b border-(--st-line) bg-(--st-panel) px-3 py-1.5">
            {move || {
                let sel = ed.selection();
                match sel.len() {
                    0 => page_tools(ed).into_any(),
                    1 => layer_tools(ed, sel.first().map(|l| l.base.id.clone()).unwrap_or_default()).into_any(),
                    _ => multi_tools(ed, sel.iter().map(|l| l.base.id.clone()).collect()).into_any(),
                }
            }}
        </div>
    }
}

fn page_tools(ed: Ed) -> impl IntoView {
    let (w, h) = ed.with_canvas_untracked_pub();
    view! {
        <span class="mr-2 text-[12.5px] text-(--st-muted)">"Page"</span>
        <select class="h-8 rounded-md border border-(--st-line) bg-(--st-panel-2) px-2 text-[12.5px]" on:change=move |e| {
            if let Some((_, pw, ph)) = PRESETS.iter().find(|(l, _, _)| *l == event_target_value(&e)) {
                ed.resize_page(*pw, *ph);
            }
        }>
            <option value="">{format!("{w} × {h} px")}</option>
            {PRESETS.map(|(label, pw, ph)| view! { <option value=label>{format!("{label} · {pw}×{ph}")}</option> }).collect_view()}
        </select>
        <span class="ml-3 text-[12px] text-(--st-muted)">"Click an element to edit it, or add one from the left."</span>
    }
}

impl Ed {
    fn with_canvas_untracked_pub(&self) -> (f64, f64) {
        self.hist.with_untracked(|h| (h.present.width, h.present.height))
    }
}

fn align_menu(ed: Ed, ids: Vec<String>) -> impl IntoView {
    let ids = StoredValue::new(ids);
    let a = move |at: PageAlign| ids.with_value(|i| ed.align(i, at));
    view! {
        <details class="relative">
            <summary class=format!("{TOOL_BTN} cursor-pointer list-none")>"Align ▾"</summary>
            <div class="absolute top-9 left-0 z-50 grid w-48 grid-cols-3 gap-1 rounded-lg border border-(--st-line) bg-(--st-panel) p-2 shadow-xl">
                {[("Left", PageAlign::Left), ("Centre", PageAlign::Center), ("Right", PageAlign::Right), ("Top", PageAlign::Top), ("Middle", PageAlign::Middle), ("Bottom", PageAlign::Bottom)]
                    .map(|(label, at)| view! { <button type="button" class=TOOL_BTN on:click=move |_| a(at)>{label}</button> }).collect_view()}
                <button type="button" class=format!("{TOOL_BTN} col-span-3") on:click=move |_| ids.with_value(|i| ed.distribute(i, true))>"Space evenly across"</button>
                <button type="button" class=format!("{TOOL_BTN} col-span-3") on:click=move |_| ids.with_value(|i| ed.distribute(i, false))>"Space evenly down"</button>
            </div>
        </details>
    }
}

fn position_menu(ed: Ed, ids: Vec<String>) -> impl IntoView {
    let ids = StoredValue::new(ids);
    view! {
        <details class="relative">
            <summary class=format!("{TOOL_BTN} cursor-pointer list-none")>"Position ▾"</summary>
            <div class="absolute top-9 right-0 z-50 w-56 rounded-lg border border-(--st-line) bg-(--st-panel) p-2 shadow-xl">
                {[("Bring to front", LayerMove::Front), ("Bring forward", LayerMove::Forward), ("Send backward", LayerMove::Backward), ("Send to back", LayerMove::Back)]
                    .map(|(label, m)| view! { <button type="button" class=format!("{TOOL_BTN} w-full justify-start") on:click=move |_| ids.with_value(|i| ed.move_layers(i, m))>{label}</button> }).collect_view()}
                <div class="my-1 border-t border-(--st-line)"></div>
                <div class="grid grid-cols-3 gap-1">
                    {[("Left", PageAlign::Left), ("Centre", PageAlign::Center), ("Right", PageAlign::Right), ("Top", PageAlign::Top), ("Middle", PageAlign::Middle), ("Bottom", PageAlign::Bottom)]
                        .map(|(label, at)| view! { <button type="button" class=TOOL_BTN on:click=move |_| ids.with_value(|i| ed.align(i, at))>{label}</button> }).collect_view()}
                </div>
            </div>
        </details>
    }
}

fn common_right(ed: Ed, ids: Vec<String>, opacity: f64, locked: bool) -> impl IntoView {
    let ids_s = StoredValue::new(ids.clone());
    view! {
        <div class="ml-auto flex items-center gap-1">
            {(!locked).then(|| position_menu(ed, ids))}
            <label class="flex items-center gap-1 text-[12px] text-(--st-muted)" title="Transparency">
                "Opacity"
                <input type="range" min="0" max="100" class="w-20" prop:value=(opacity * 100.0).round().to_string()
                    on:input=move |e| {
                        let v = event_target_value(&e).parse::<f64>().unwrap_or(100.0) / 100.0;
                        ids_s.with_value(|i| ed.patch_each(i, |l| l.base.opacity = v, Some(&format!("opacity:{}", i.join(",")))));
                    } />
            </label>
            <ToolButton title=if locked { "Unlock" } else { "Lock" } active=locked on_click=move || ids_s.with_value(|i| ed.patch_each(i, |l| l.base.locked = !locked, None))>
                {if locked { "🔒" } else { "🔓" }}
            </ToolButton>
            <ToolButton title="Duplicate (Ctrl+D)" on_click=move || ids_s.with_value(|i| ed.duplicate(i))>"⧉"</ToolButton>
            <ToolButton title="Delete (Del)" on_click=move || ids_s.with_value(|i| ed.remove(i))>"🗑"</ToolButton>
        </div>
    }
}

fn multi_tools(ed: Ed, ids: Vec<String>) -> impl IntoView {
    let n = ids.len();
    let sel = ed.selection();
    let all_locked = sel.iter().all(|l| l.base.locked);
    let opacity = sel.first().map_or(1.0, |l| l.base.opacity);
    view! {
        <span class="mr-2 rounded-md bg-(--st-accent-soft) px-2 py-1 text-[12px]">{format!("{n} elements selected")}</span>
        {align_menu(ed, ids.clone())}
        <button type="button" class="ml-1 text-[12px] text-(--st-muted) underline hover:text-(--st-text)" on:click=move |_| ed.select(None)>"Clear selection"</button>
        {common_right(ed, ids, opacity, all_locked)}
    }
}

fn layer_tools(ed: Ed, id: String) -> impl IntoView {
    let Some(l) = ed.selected_one() else { return ().into_any() };
    let locked = l.base.locked;
    let opacity = l.base.opacity;
    let ids = vec![id.clone()];
    let idv = StoredValue::new(id);
    let tweak = move |key: &'static str, f: Box<dyn FnOnce(&mut lattice_core::canvas::Layer)>| {
        idv.with_value(|i| ed.tweak(i, key, f))
    };
    let specific = if locked {
        view! { <span class="text-[12px] text-(--st-muted)">"This element is locked."</span> }.into_any()
    } else {
        match &l.shape {
            Shape::Text(t) => {
                let t = t.clone();
                let style = t.font_style;
                view! {
                    <select class="h-8 max-w-44 rounded-md border border-(--st-line) bg-(--st-panel-2) px-2 text-[12.5px]" title="Font"
                        on:change=move |e| { let v = event_target_value(&e); tweak("font", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.font_family = v })) }>
                        {FONTS.map(|(f, _)| view! { <option value=f selected=f == t.font_family style=format!("font-family: \"{f}\"")>{f}</option> }).collect_view()}
                    </select>
                    {num_input(t.font_size, "1", "w-16", "Font size", move |v| if v > 0.0 { tweak("size", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.font_size = v })) })}
                    {color_input(t.fill.main_color().to_string(), "Text colour", move |c| tweak("fill", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.fill = match &t.fill { Fill::Linear { to, angle, .. } => Fill::Linear { from: c, to: to.clone(), angle: *angle }, Fill::Solid { .. } => Fill::Solid { color: c } } })))}
                    <ToolButton title="Bold" active=style.bold() on_click=move || tweak("style", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.font_style = FontStyle::of(!style.bold(), style.italic()) }))><b>"B"</b></ToolButton>
                    <ToolButton title="Italic" active=style.italic() on_click=move || tweak("style", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.font_style = FontStyle::of(style.bold(), !style.italic()) }))><i>"I"</i></ToolButton>
                    <ToolButton title="Underline" active={t.text_decoration == Decoration::Underline} on_click=move || tweak("decoration", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.text_decoration = if t.text_decoration == Decoration::Underline { Decoration::None } else { Decoration::Underline } }))><u>"U"</u></ToolButton>
                    {[(Align::Left, "⇤"), (Align::Center, "↔"), (Align::Right, "⇥")].map(|(a, g)| view! {
                        <ToolButton title=format!("Align {a:?}").to_lowercase() active={t.align == a} on_click=move || tweak("align", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.align = a }))>{g}</ToolButton>
                    }).collect_view()}
                    <span class="ml-1 text-[11px] text-(--st-muted)">"Spacing"</span>
                    {num_input(t.letter_spacing, "0.5", "w-14", "Letter spacing", move |v| tweak("spacing", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.letter_spacing = v })))}
                    <span class="text-[11px] text-(--st-muted)">"Line"</span>
                    {num_input(t.line_height, "0.05", "w-14", "Line height", move |v| if v > 0.0 { tweak("line", Box::new(move |l| if let Shape::Text(t) = &mut l.shape { t.line_height = v })) })}
                }.into_any()
            }
            Shape::Image(i) => {
                let contain = i.fit == Fit::Contain;
                let product = i.product_id.is_some();
                let tol = RwSignal::new(40.0);
                view! {
                    <ToolButton title="Replace with another product photo" on_click=move || { ed.swap_id.set(Some(idv.get_value())); ed.tab.set(Some(Tab::Products)); }>"Replace"</ToolButton>
                    <ToolButton title="Fit the photo inside its frame" active=contain on_click=move || tweak("fit", Box::new(move |l| if let Shape::Image(i) = &mut l.shape { i.fit = if i.fit == Fit::Contain { Fit::Stretch } else { Fit::Contain } }))>"Fit inside"</ToolButton>
                    <ToolButton title="Flip horizontally" on_click=move || ed.flip(&idv.get_value(), true)>"⇋"</ToolButton>
                    <ToolButton title="Flip vertically" on_click=move || ed.flip(&idv.get_value(), false)>"⇵"</ToolButton>
                    <ToolButton title="Fit to page" on_click=move || ed.fit_to_page(&idv.get_value())>"Fit to page"</ToolButton>
                    <super::magic::MagicTools ed=ed id=idv.get_value() />
                    {product.then(|| view! {
                        <span class="ml-2 text-[11px] text-(--st-muted)">"Tolerance"</span>
                        <input type="range" min="5" max="120" class="w-20" prop:value=move || tol.get().to_string() on:input=move |e| tol.set(event_target_value(&e).parse().unwrap_or(40.0)) />
                        <button type="button" class=TOOL_BTN disabled=move || ed.removing.get() on:click=move |_| ed.cut_out(idv.get_value(), tol.get_untracked())>
                            {move || if ed.removing.get() { "Removing…" } else { "Remove background" }}
                        </button>
                    })}
                }.into_any()
            }
            Shape::Line(_) | Shape::Curve(_) => {
                let Some(x) = l.line().cloned() else { return ().into_any() };
                let dashed = x.dash.as_ref().is_some_and(|d| !d.is_empty());
                view! {
                    {color_input(x.stroke.clone(), "Line colour", move |c| tweak("stroke", Box::new(move |l| if let Some(x) = l.line_mut() { x.stroke = c })))}
                    <span class="text-[11px] text-(--st-muted)">"Width"</span>
                    {num_input(x.stroke_width, "1", "w-14", "Line width", move |v| if v >= 1.0 { tweak("width", Box::new(move |l| if let Some(x) = l.line_mut() { x.stroke_width = v })) })}
                    <ToolButton title="Dashed" active=dashed on_click=move || tweak("dash", Box::new(move |l| if let Some(x) = l.line_mut() { x.dash = if x.dash.as_ref().is_some_and(|d| !d.is_empty()) { None } else { Some(vec![16.0, 12.0]) } }))>"Dashed"</ToolButton>
                    <select class="h-8 rounded-md border border-(--st-line) bg-(--st-panel-2) px-2 text-[12.5px]" title="Line ends"
                        on:change=move |e| { let cap = match event_target_value(&e).as_str() { "butt" => LineCap::Butt, "square" => LineCap::Square, _ => LineCap::Round }; tweak("cap", Box::new(move |l| if let Some(x) = l.line_mut() { x.line_cap = cap })) }>
                        {[("round", "Round ends"), ("butt", "Flat ends"), ("square", "Square ends")].map(|(v, label)| view! { <option value=v selected={x.line_cap.css() == v}>{label}</option> }).collect_view()}
                    </select>
                    <ToolButton title="Snap to right angles (Shift: 45°)" active=ed.snap_right on_click=move || ed.snap_right.update(|s| *s = !*s)>"Snap ⟂"</ToolButton>
                }.into_any()
            }
            _ => {
                let Some(s) = l.style().cloned() else { return ().into_any() };
                let rect = matches!(&l.shape, Shape::Rect(_));
                let radius = match &l.shape {
                    Shape::Rect(r) => r.corner_radius,
                    _ => 0.0,
                };
                view! {
                    <span class="text-[11px] text-(--st-muted)">"Fill"</span>
                    {color_input(s.fill.main_color().to_string(), "Fill colour", move |c| tweak("fill", Box::new(move |l| if let Some(s) = l.style_mut() { s.fill = match &s.fill { Fill::Linear { to, angle, .. } => Fill::Linear { from: c, to: to.clone(), angle: *angle }, Fill::Solid { .. } => Fill::Solid { color: c } } })))}
                    <span class="ml-2 text-[11px] text-(--st-muted)">"Outline"</span>
                    {color_input(s.stroke.clone(), "Outline colour", move |c| tweak("stroke", Box::new(move |l| if let Some(s) = l.style_mut() { s.stroke = c })))}
                    {num_input(s.stroke_width, "1", "w-14", "Outline width", move |v| if v >= 0.0 { tweak("strokeWidth", Box::new(move |l| if let Some(s) = l.style_mut() { s.stroke_width = v })) })}
                    {rect.then(|| view! {
                        <span class="ml-2 text-[11px] text-(--st-muted)">"Corners"</span>
                        {num_input(radius, "1", "w-14", "Corner radius", move |v| if v >= 0.0 { tweak("radius", Box::new(move |l| if let Shape::Rect(r) = &mut l.shape { r.corner_radius = v })) })}
                    })}
                }.into_any()
            }
        }
    };
    view! { {specific} {common_right(ed, ids, opacity, locked)} }.into_any()
}

// --------------------------------------------------------------- layers

#[component]
pub fn LayersPanel(ed: Ed) -> impl IntoView {
    let drag_id = RwSignal::new(None::<String>);
    let over_id = RwSignal::new(None::<String>);
    let renaming = RwSignal::new(None::<String>);
    view! {
        <section class="flex min-h-0 flex-1 flex-col">
            <header class="flex h-10 shrink-0 items-center gap-2 border-b border-(--st-line) px-3">
                <span class="font-semibold">"Layers"</span>
                <span class="ml-auto text-[11px] text-(--st-muted)">{move || ed.with_canvas(|c| c.layers.len())}</span>
            </header>
            <ul class="min-h-0 flex-1 overflow-y-auto py-1" on:dragleave=move |_| over_id.set(None)>
                {move || {
                    let layers = ed.with_canvas(|c| c.layers.clone());
                    if layers.is_empty() {
                        return view! { <li class="px-3 py-6 text-center text-[12px] text-(--st-muted)">"Nothing here yet. Add products, text or shapes from the left."</li> }.into_any();
                    }
                    let sel = ed.selected.get();
                    let order: Vec<String> = layers.iter().map(|l| l.base.id.clone()).collect();
                    let order = StoredValue::new(order);
                    layers.iter().rev().map(|l| {
                        let id = l.base.id.clone();
                        let selected = sel.contains(&id);
                        let label = layer_label(l);
                        let (visible, locked) = (l.base.visible, l.base.locked);
                        let swatch = match &l.shape {
                            Shape::Image(i) => format!("background: center / contain no-repeat url('{}')", thumb_url(&i.url)),
                            Shape::Text(t) => format!("background: #fff; color: {}", t.fill.main_color()),
                            _ => l.style().map(|s| format!("background: {}", fill_css(&s.fill))).or_else(|| l.line().map(|x| format!("background: linear-gradient(transparent 45%, {} 45%, {} 55%, transparent 55%)", x.stroke, x.stroke))).unwrap_or_default(),
                        };
                        let is_text = l.kind() == "text";
                        let (i1, i2, i3, i4, i6) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
                        view! {
                            <li draggable=move || (renaming.get().as_deref() != Some(i6.as_str())).to_string()
                                on:dragstart={ let id = id.clone(); move |e: web_sys::DragEvent| { drag_id.set(Some(id.clone())); if let Some(dt) = e.data_transfer() { dt.set_effect_allowed("move"); } } }
                                on:dragend=move |_| { drag_id.set(None); over_id.set(None); }
                                on:dragover={ let id = id.clone(); move |e: web_sys::DragEvent| if drag_id.get_untracked().is_some() { e.prevent_default(); over_id.set(Some(id.clone())); } }
                                on:drop={ let id = id.clone(); move |e: web_sys::DragEvent| {
                                    e.prevent_default();
                                    if let Some(d) = drag_id.get_untracked()
                                        && let Some(to) = order.with_value(|o| o.iter().position(|x| *x == id))
                                    {
                                        ed.reorder(&d, to);
                                    }
                                    drag_id.set(None);
                                    over_id.set(None);
                                } }
                                on:click={ let id = id.clone(); move |e: web_sys::MouseEvent| {
                                    let cur = ed.selected.get_untracked();
                                    if e.shift_key() && !cur.is_empty() {
                                        order.with_value(|o| {
                                            let anchor = cur.last().and_then(|a| o.iter().position(|x| x == a)).unwrap_or(0);
                                            let here = o.iter().position(|x| *x == id).unwrap_or(0);
                                            let (a, b) = if anchor < here { (anchor, here) } else { (here, anchor) };
                                            ed.select_many(o.get(a..=b).map(<[String]>::to_vec).unwrap_or_default());
                                        });
                                    } else if e.ctrl_key() || e.meta_key() {
                                        ed.toggle(&id);
                                    } else {
                                        ed.select(Some(id.clone()));
                                    }
                                } }
                                class=move || format!("group mx-1 flex cursor-default items-center gap-2 rounded-md border-t-2 border-transparent px-1.5 py-1 {} {} {} {}",
                                    if selected { "bg-(--st-accent-soft)" } else { "hover:bg-(--st-hover)" },
                                    if over_id.get().as_deref() == Some(i1.as_str()) && drag_id.get().as_deref() != Some(i1.as_str()) { "border-t-(--st-accent)" } else { "" },
                                    if drag_id.get().as_deref() == Some(i1.as_str()) { "opacity-40" } else { "" },
                                    if visible { "" } else { "opacity-50" })>
                                <span class="checkerboard flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded border border-(--st-line) text-[11px] font-bold" style=swatch>
                                    {is_text.then_some("T")}
                                </span>
                                {move || if renaming.get().as_deref() == Some(i2.as_str()) {
                                    let id = i2.clone();
                                    view! {
                                        <input class="h-6 min-w-0 flex-1 rounded border border-(--st-accent) bg-(--st-panel-2) px-1 text-[12px] outline-none" prop:value=label.clone()
                                            on:blur=move |e| { let v = event_target_value(&e).trim().to_string(); ed.patch(&id, |l| l.base.name = (!v.is_empty()).then_some(v), None); renaming.set(None); }
                                            on:keydown=move |e| { if e.key() == "Enter" { if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = t.blur(); } } else if e.key() == "Escape" { renaming.set(None); } } />
                                    }.into_any()
                                } else {
                                    let id = i2.clone();
                                    view! { <span class="min-w-0 flex-1 truncate text-[12.5px]" title="Double-click to rename" on:dblclick=move |_| renaming.set(Some(id.clone()))>{label.clone()}</span> }.into_any()
                                }}
                                <button type="button" title=if locked { "Unlock" } else { "Lock" } class=format!("h-6 w-6 rounded text-[12px] hover:bg-(--st-hover) {}", if locked { "" } else { "opacity-0 group-hover:opacity-100" })
                                    on:click=move |e| { e.stop_propagation(); ed.patch(&i3, |l| l.base.locked = !locked, None); }>{if locked { "🔒" } else { "🔓" }}</button>
                                <button type="button" title=if visible { "Hide" } else { "Show" } class=format!("h-6 w-6 rounded text-[12px] hover:bg-(--st-hover) {}", if visible { "opacity-0 group-hover:opacity-100" } else { "" })
                                    on:click=move |e| { e.stop_propagation(); ed.patch(&i4, |l| l.base.visible = !visible, None); }>{if visible { "👁" } else { "◌" }}</button>
                            </li>
                        }
                    }).collect_view().into_any()
                }}
            </ul>
        </section>
    }
}

/// Position, size and rotation of one selected element, typed exactly.
#[component]
pub fn TransformPanel(ed: Ed) -> impl IntoView {
    view! {
        <section class="shrink-0 border-t border-(--st-line) p-3">
            {move || match ed.selected_one() {
                None => view! { <p class="text-[12px] text-(--st-muted)">"Select one element to set its position, size and rotation."</p> }.into_any(),
                Some(l) => {
                    let id = StoredValue::new(l.base.id.clone());
                    let r = super::render::client_rect(&l);
                    let b = l.base.clone();
                    let field = move |label: &'static str, value: f64, on: Box<dyn Fn(f64)>| view! {
                        <label class="flex items-center gap-1 text-[11px] text-(--st-muted)">
                            <span class="w-4">{label}</span>
                            {num_input(value, "1", "w-full", label, on)}
                        </label>
                    };
                    view! {
                        <div class="grid grid-cols-2 gap-2">
                            {field("X", b.x, Box::new(move |v| id.with_value(|i| ed.tweak(i, "x", |l| l.base.x = v))))}
                            {field("Y", b.y, Box::new(move |v| id.with_value(|i| ed.tweak(i, "y", |l| l.base.y = v))))}
                            {field("W", r.width, Box::new(move |v| if v > 0.0 { id.with_value(|i| ed.tweak(i, "w", |l| { let w = super::render::client_rect(l).width; if w > 0.0 { l.base.scale_x *= v / w; } })) }))}
                            {field("H", r.height, Box::new(move |v| if v > 0.0 { id.with_value(|i| ed.tweak(i, "h", |l| { let h = super::render::client_rect(l).height; if h > 0.0 { l.base.scale_y *= v / h; } })) }))}
                            {field("°", b.rotation, Box::new(move |v| id.with_value(|i| ed.tweak(i, "rotation", |l| l.base.rotation = v))))}
                        </div>
                    }.into_any()
                }
            }}
        </section>
    }
}

// ------------------------------------------------------- bars and menus

#[component]
pub fn Topbar(ed: Ed) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let (title, subtitle, back) = ed.config.with_value(|c| (c.title.clone(), c.subtitle.clone(), c.back_href.clone()));
    let back = StoredValue::new(back);
    let leave = move || {
        if ed.dirty() && !crate::kit::confirm("You have unsaved changes. Leave without saving?") {
            return;
        }
        navigate.with_value(|nav| back.with_value(|b| nav(b, Default::default())));
    };
    let can_undo = move || ed.hist.with(|h| !h.past.is_empty());
    let can_redo = move || ed.hist.with(|h| !h.future.is_empty());
    view! {
        <header class="studio-topbar flex h-13 shrink-0 items-center gap-2 px-3">
            <button type="button" class=TOOL_BTN title="Back" on:click=move |_| leave()>"← Back"</button>
            <div class="min-w-0">
                <div class="truncate text-[14px] font-semibold">{title}</div>
                {(!subtitle.is_empty()).then(|| view! { <div class="truncate text-[11.5px] text-(--st-muted)">{subtitle}</div> })}
            </div>
            <div class="ml-4 flex items-center gap-1">
                <button type="button" class=TOOL_BTN title="Undo (Ctrl+Z)" disabled=move || !can_undo() on:click=move |_| ed.undo()>"↶"</button>
                <button type="button" class=TOOL_BTN title="Redo (Ctrl+Shift+Z)" disabled=move || !can_redo() on:click=move |_| ed.redo()>"↷"</button>
            </div>
            <div class="ml-auto flex items-center gap-2">
                <span class="text-[12px] text-(--st-muted)">{move || if ed.dirty() { "Unsaved changes" } else { "All changes saved" }}</span>
                <button type="button" class=TOOL_BTN title="Download as PNG" on:click=move |_| ed.download()>"Download"</button>
                <button type="button" class="rounded-md bg-(--st-gold) px-4 py-1.5 text-[13px] font-semibold text-(--st-accent-strong) hover:brightness-95 disabled:opacity-60"
                    disabled=move || ed.saving.get() on:click=move |_| spawn_local(async move { ed.save().await; })>
                    {move || if ed.saving.get() { "Saving…" } else { "Save" }}
                </button>
            </div>
        </header>
    }
}

#[component]
pub fn Footer(ed: Ed) -> impl IntoView {
    view! {
        <footer class="flex h-10 shrink-0 items-center gap-2 border-t border-(--st-line) bg-(--st-panel) px-3 text-[12px]">
            <button type="button" class=TOOL_BTN title="Zoom out (Ctrl −)" on:click=move |_| ed.zoom_to(|z| z / 1.25, None)>"−"</button>
            <span class="w-12 text-center tabular-nums">{move || format!("{}%", (ed.zoom() * 100.0).round())}</span>
            <button type="button" class=TOOL_BTN title="Zoom in (Ctrl +)" on:click=move |_| ed.zoom_to(|z| z * 1.25, None)>"+"</button>
            <button type="button" class=TOOL_BTN title="Fit (Ctrl 0)" on:click=move |_| ed.zoom_fit()>"Fit"</button>
            <span class="ml-3 text-(--st-muted) tabular-nums">{move || ed.with_canvas(|c| format!("{} × {} px", c.width, c.height))}</span>
            <span class="ml-auto text-(--st-muted)">{move || ed.with_canvas(|c| format!("{} layer{}", c.layers.len(), if c.layers.len() == 1 { "" } else { "s" }))}</span>
            <span class="hidden text-(--st-muted) xl:inline">" · Space+drag to pan · Ctrl+wheel to zoom · Alt while dragging skips snapping"</span>
        </footer>
    }
}

/// The deck's slides under the page; switching saves this one first.
#[component]
pub fn PageStrip(ed: Ed) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let (pages, current) = ed.config.with_value(|c| (c.pages.clone(), c.page_id.clone()));
    view! {
        <div class="flex h-24 shrink-0 items-center gap-2 overflow-x-auto border-t border-(--st-line) bg-(--st-panel-2) px-3">
            {pages.into_iter().enumerate().map(|(i, p)| {
                let here = current.as_deref() == Some(p.id.as_str());
                let href = p.href.clone();
                let saved_canvas = p.canvas.clone();
                let thumb = Signal::derive(move || if here { ed.saved.get() } else { saved_canvas.clone() });
                view! {
                    <button type="button" title=format!("Slide {}", i + 1)
                        class=format!("relative shrink-0 rounded border-2 {}", if here { "border-(--st-accent)" } else { "border-transparent hover:border-(--st-line)" })
                        on:click=move |_| {
                            if here {
                                return;
                            }
                            let href = href.clone();
                            spawn_local(async move {
                                if ed.dirty() && !ed.save().await {
                                    return;
                                }
                                navigate.with_value(|nav| nav(&href, Default::default()));
                            });
                        }>
                        <Thumb canvas=thumb width=112.0 />
                        <span class="absolute bottom-0.5 left-1 rounded bg-black/50 px-1 text-[10px] text-white">{i + 1}</span>
                    </button>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn ContextMenu(ed: Ed) -> impl IntoView {
    crate::kit::on_window(leptos::ev::pointerdown, move |e| {
        let inside = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("[data-studio-menu]").ok().flatten())
            .is_some();
        if !inside {
            ed.menu.set(None);
        }
    });
    let item = move |label: &'static str, hint: &'static str, run: Box<dyn Fn()>| {
        view! {
            <button type="button" class="flex w-full items-center justify-between rounded px-3 py-1.5 text-left text-[12.5px] hover:bg-(--st-hover)"
                on:click=move |_| { run(); ed.menu.set(None); }>
                <span>{label}</span><span class="text-[11px] text-(--st-muted)">{hint}</span>
            </button>
        }
    };
    view! {
        {move || ed.menu.get().map(|(x, y)| {
            let ids = ed.selected.get_untracked();
            let ids = StoredValue::new(ids);
            let has = ids.with_value(|i| !i.is_empty());
            let locked = ed.selection().iter().all(|l| l.base.locked);
            view! {
                <div data-studio-menu="" class="fixed z-[70] w-56 rounded-lg border border-(--st-line) bg-(--st-panel) py-1 shadow-2xl" style=format!("left: {x}px; top: {y}px")>
                    {has.then(|| view! {
                        {item("Copy", "Ctrl+C", Box::new(move || ed.copy()))}
                        {item("Duplicate", "Ctrl+D", Box::new(move || ids.with_value(|i| ed.duplicate(i))))}
                        {item("Delete", "Del", Box::new(move || ids.with_value(|i| ed.remove(i))))}
                        <div class="my-1 border-t border-(--st-line)"></div>
                        {item("Bring to front", "Ctrl+Shift+]", Box::new(move || ids.with_value(|i| ed.move_layers(i, LayerMove::Front))))}
                        {item("Bring forward", "Ctrl+]", Box::new(move || ids.with_value(|i| ed.move_layers(i, LayerMove::Forward))))}
                        {item("Send backward", "Ctrl+[", Box::new(move || ids.with_value(|i| ed.move_layers(i, LayerMove::Backward))))}
                        {item("Send to back", "Ctrl+Shift+[", Box::new(move || ids.with_value(|i| ed.move_layers(i, LayerMove::Back))))}
                        <div class="my-1 border-t border-(--st-line)"></div>
                        {item(if locked { "Unlock" } else { "Lock" }, "", Box::new(move || ids.with_value(|i| ed.patch_each(i, |l| l.base.locked = !locked, None))))}
                    })}
                    {item("Paste", "Ctrl+V", Box::new(move || ed.paste()))}
                    {item("Select all", "Ctrl+A", Box::new(move || ed.select_all()))}
                    {item("Zoom to fit", "Ctrl+0", Box::new(move || ed.zoom_fit()))}
                    {item("Centre the page", "", Box::new(move || ed.recenter()))}
                </div>
            }
        })}
    }
}
