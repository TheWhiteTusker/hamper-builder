//! The calculator's screen (ports of `calculator-view.tsx`, `product-details-card.tsx`,
//! `code-field.tsx`, `color-photo-rows.tsx`, `stage-section.tsx`, `stage-line-row.tsx`,
//! `cells.tsx`, `hierarchy-cells.tsx`, `cost-summary.tsx`, `pricing-boxes.tsx`,
//! `apply-price-dialog.tsx`, `leave-guard.tsx` and the photo gallery).

use super::save::{Payload, save_costing};
use super::{BOUGHT_OUT, Calc, ORIGINS, Pending, UploadStatus, num_text, saved_lines};
use crate::db;
use crate::kit::{ImagePreview, confirm, costing_href, product_href};
use crate::pages::product::CostBreakdown;
use crate::photos;
use lattice_core::Decimal;
use lattice_core::colors::{FALLBACK_COLOR_HEX, ProductColor};
use lattice_core::cost_tree::StageNode;
use lattice_core::costing::{ProductCostLine, is_sized_unit, is_time_unit, with_overhead};
use lattice_core::dimensions::COMMON_UNITS;
use lattice_core::images::ProductImage;
use lattice_core::js::{self, dec, f};
use lattice_core::money::{format_money, format_pct};
use lattice_core::mrp::mrp_margin;
use lattice_core::numbers::parse_num;
use lattice_core::product_code::codes_for_colors;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[component]
pub fn Tabs(master: bool) -> impl IntoView {
    let tab = |active: bool| {
        format!(
            "rounded-md px-3 py-1.5 text-xs font-semibold transition-all {}",
            if active {
                "bg-white text-(--color-brand-dark) shadow-sm"
            } else {
                "text-(--color-muted) hover:text-(--color-ink)"
            }
        )
    };
    let here = window().location().pathname().unwrap_or_default();
    view! {
        <div class="flex items-center gap-1 rounded-lg bg-slate-100 p-1">
            <A href=if master { "/products/new".to_string() } else { here } attr:class=tab(!master)>
                {if master { "New product" } else { "Costing" }}
            </A>
            <A href="/cost-calculator/master" attr:class=tab(master)>"Rates"</A>
        </div>
    }
}

/// A number box that keeps what is typed ("1." stays "1.") and only rewrites
/// itself when the value changes from outside, as React's number inputs do.
#[component]
fn NumInput(
    #[prop(into)] value: Signal<Option<Decimal>>,
    on_change: impl Fn(Option<Decimal>) + 'static,
    #[prop(optional)] nullable: bool,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] min: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional, into)] label: Option<String>,
) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let v = value.get();
        if let Some(el) = input.get() {
            let shown = el.value();
            let same = match v {
                None => shown.is_empty(),
                Some(d) => !shown.is_empty() && dec(js::number(&shown)) == d,
            };
            if !same {
                el.set_value(&v.map(num_text).unwrap_or_default());
            }
        }
    });
    view! {
        <input
            node_ref=input
            type="number"
            step="any"
            min=min
            placeholder=placeholder
            aria-label=label
            class=format!("input input-num py-1 px-2 text-xs {class}")
            on:input=move |e| {
                let raw = event_target_value(&e);
                on_change(if nullable && raw.is_empty() { None } else { Some(dec(js::number(&raw))) });
            }
        />
    }
}

#[component]
pub fn CalculatorView(calc: Calc) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let initial = StoredValue::new(calc.snapshot());
    let dirty = Memo::new(move |_| calc.snapshot() != initial.get_value());
    // Some(None): going back; Some(Some(path)): following a link.
    let leaving = RwSignal::new(None::<Option<String>>);
    let fallback = calc
        .initial
        .with_value(|c| c.as_ref().map_or_else(|| "/products".to_string(), |c| product_href(&c.product.code)));

    let go = StoredValue::new_local(move |to: Option<String>| {
        navigate.with_value(|nav| match to {
            Some(path) => nav(&path, Default::default()),
            None => {
                let h = window().history().ok();
                if h.as_ref().and_then(|h| h.length().ok()).unwrap_or(0) > 1 {
                    if let Some(h) = h {
                        let _ = h.back();
                    }
                } else {
                    nav(&fallback, Default::default());
                }
            }
        });
    });

    // Leaving with unsaved changes asks first: every in-app link, caught
    // before the router's own handler runs.
    let listener = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if !dirty.get_untracked() || e.button() != 0 || e.meta_key() || e.ctrl_key() || e.shift_key() || e.alt_key() {
            return;
        }
        let anchor = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|el| el.closest("a[href]").ok().flatten());
        let Some(a) = anchor.and_then(|a| a.dyn_into::<web_sys::HtmlAnchorElement>().ok()) else { return };
        if !a.target().is_empty() || a.has_attribute("download") {
            return;
        }
        let here = window().location();
        if a.origin() != here.origin().unwrap_or_default() || a.pathname() == here.pathname().unwrap_or_default() {
            return;
        }
        e.prevent_default();
        e.stop_propagation();
        leaving.set(Some(Some(format!("{}{}{}", a.pathname(), a.search(), a.hash()))));
    });
    let _ = document().add_event_listener_with_callback_and_bool("click", listener.as_ref().unchecked_ref(), true);
    let listener = StoredValue::new_local(listener);
    on_cleanup(move || {
        listener.with_value(|l| {
            let _ = document().remove_event_listener_with_callback_and_bool("click", l.as_ref().unchecked_ref(), true);
        });
    });

    let save_now = move || start_save(calc, navigate);

    view! {
        <div class="space-y-6">
            {calc.editing.then(|| view! {
                <button
                    type="button"
                    on:click=move |_| if dirty.get_untracked() { leaving.set(Some(None)) } else { go.with_value(|g| g(None)) }
                    class="-mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)"
                >
                    <span aria-hidden="true">"←"</span>
                    " Back"
                </button>
            })}
            {move || leaving.get().map(|to| view! {
                <LeaveDialog
                    calc=calc
                    on_save=move || save_now()
                    on_discard=move || {
                        leaving.set(None);
                        initial.set_value(calc.snapshot());
                        let to = to.clone();
                        go.with_value(|g| g(to));
                    }
                    on_cancel=move || leaving.set(None)
                />
            })}
            <DetailsCard calc=calc />
            {move || {
                if calc.product_id.get().is_empty() {
                    view! {
                        <div class="card border-dashed bg-slate-50/60 p-4 text-center text-xs text-(--color-muted)">
                            "Save this product first to upload and manage photos for each of its color finishes."
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <div class="card p-5 shadow-sm"><Gallery calc=calc /></div> }.into_any()
                }
            }}
            {calc.stages.get_value().into_iter().map(|stage| view! { <StageSection calc=calc stage=stage /> }).collect_view()}
            <Summary calc=calc on_save=move || save_now() />
            {move || calc.asking_price.get().then(|| view! { <ApplyPriceDialog calc=calc navigate=navigate /> })}
        </div>
    }
}

fn nav_box<N>(n: StoredValue<N, LocalStorage>) -> impl Fn(&str) + Copy
where
    N: Fn(&str, leptos_router::NavigateOptions) + 'static,
{
    move |path: &str| n.with_value(|nav| nav(path, Default::default()))
}

/// The price-change question is asked when other colours exist and the price moved.
fn price_question(calc: Calc) -> Option<(String, String, Decimal, Decimal)> {
    let product = calc.initial.with_value(|c| c.as_ref().map(|c| c.product.clone()))?;
    if calc.siblings.with_value(Vec::is_empty) {
        return None;
    }
    let new_sp = calc.effective_sp();
    let changed = new_sp != product.default_sp || calc.totals.get_untracked().total_cost != product.cost_price;
    changed.then(|| {
        (product.code.clone(), product.colors.first().cloned().unwrap_or_default(), product.default_sp, new_sp)
    })
}

fn start_save<N>(calc: Calc, navigate: StoredValue<N, LocalStorage>)
where
    N: Fn(&str, leptos_router::NavigateOptions) + 'static,
{
    calc.error.set(None);
    calc.success.set(None);
    if calc.code.get_untracked().trim().is_empty() {
        return calc.error.set(Some("Product Code is required.".into()));
    }
    if calc.name.get_untracked().trim().is_empty() {
        return calc.error.set(Some("Product Name is required.".into()));
    }
    if calc.colors.with_untracked(Vec::is_empty) {
        return calc.error.set(Some("Pick a colour: every product is one colour.".into()));
    }
    if calc.over_mrp() {
        let msg = format!(
            "SP {} is above the MRP limit of {} (MRP includes GST). Save anyway?",
            format_money(Some(calc.effective_sp())),
            format_money(calc.mrp_max.get_untracked())
        );
        if !confirm(&msg) {
            return;
        }
    }
    if price_question(calc).is_some() {
        return calc.asking_price.set(true);
    }
    commit(calc, Vec::new(), nav_box(navigate));
}

fn me_id() -> String {
    match expect_context::<crate::state::Auth>().phase.get_untracked() {
        crate::state::Phase::SignedIn(info) => info.user.map(|u| u.id).unwrap_or_default(),
        _ => String::new(),
    }
}

fn commit(calc: Calc, apply_to: Vec<String>, navigate: impl Fn(&str) + 'static) {
    let code = calc.code.get_untracked().trim().to_string();
    let payload = Payload {
        sheet_id: calc.initial.with_value(|c| c.as_ref().and_then(|c| c.sheet.as_ref().map(|s| s.sheet.id.clone()))),
        product_id: Some(calc.product_id.get_untracked()).filter(|s| !s.is_empty()),
        code: code.clone(),
        name: calc.name.get_untracked().trim().to_string(),
        category_id: Some(calc.category_id.get_untracked()).filter(|s| !s.is_empty()),
        source: Some(calc.source.get_untracked().trim().to_string()).filter(|s| !s.is_empty()),
        colors: calc.colors.get_untracked(),
        is_active: calc.is_active.get_untracked(),
        markup_pct: parse_num(&calc.markup.get_untracked()),
        stage_overheads: calc.stage_overheads(),
        selling_price: calc.effective_sp(),
        notes: Some(calc.notes.get_untracked().trim().to_string()).filter(|s| !s.is_empty()),
        lines: saved_lines(&calc.lines.get_untracked()),
        apply_to,
        user_id: me_id(),
    };
    calc.busy.set(true);
    spawn_local(async move {
        let res = save_costing(payload).await;
        calc.asking_price.set(false);
        let saved = match res {
            Ok(s) => s,
            Err(e) => {
                calc.busy.set(false);
                return calc.error.set(Some(e.to_string()));
            }
        };
        let ids: BTreeMap<String, String> = saved.variants.iter().map(|v| (v.color.clone(), v.id.clone())).collect();
        let (failed, message) = after_save(calc, ids, &saved.product_id).await;
        let mut stale = vec!["products".to_string(), "catalog".into(), format!("product:{code}"), "dashboard".into()];
        stale.extend(saved.variants.iter().map(|v| format!("product:{}", v.code)));
        let keys: Vec<&str> = stale.iter().map(String::as_str).collect();
        db::cache_drop(&keys).await;
        calc.busy.set(false);
        if !failed {
            return navigate(&product_href(&code));
        }
        // A photo failed: stay put so the error is visible and can be retried.
        let codes = if saved.variants.is_empty() {
            code
        } else {
            saved.variants.iter().map(|v| v.code.clone()).collect::<Vec<_>>().join(", ")
        };
        calc.success.set(Some(format!(
            "Saved {codes} with CP {} and SP {}.{message}",
            format_money(Some(saved.total_cost)),
            format_money(Some(saved.selling_price))
        )));
        calc.product_id.set(saved.product_id);
    });
}

/// Uploads one colour's files to `pid`. Returns what landed and the first error.
async fn upload_files(
    calc: Calc,
    pid: &str,
    color: &str,
    files: Vec<web_sys::File>,
    existing: usize,
    label: &dyn Fn(usize) -> String,
) -> (Vec<ProductImage>, Option<String>) {
    let mut added = Vec::new();
    for (i, file) in files.iter().enumerate() {
        calc.upload.set(Some(UploadStatus { color: color.into(), busy: true, ok: Some(label(i)), error: None }));
        // The first photo of a product with none becomes its cover.
        match photos::upload_product_image(file, pid, Some(color), existing + added.len() == 0 && i == 0).await {
            Ok(img) => added.push(img),
            Err(e) => return (added, Some(e.to_string())),
        }
    }
    (added, None)
}

async fn upload_for_color(calc: Calc, color: String, files: Vec<web_sys::File>) {
    let pid = calc.product_id.get_untracked();
    if pid.is_empty() {
        let n = files.len();
        let queued: Vec<Pending> = files
            .into_iter()
            .map(|file| Pending {
                key: calc.key(),
                color: color.clone(),
                url: web_sys::Url::create_object_url_with_blob(&file).unwrap_or_default(),
                file,
            })
            .collect();
        calc.pending.update(|p| p.extend(queued));
        return calc.upload.set(Some(UploadStatus {
            color,
            ok: Some(format!("{n} will upload on save")),
            ..Default::default()
        }));
    }
    let pid = calc.variant_ids.with_untracked(|v| v.get(&color).cloned()).unwrap_or(pid);
    let existing = calc.images.with_untracked(|imgs| imgs.iter().filter(|i| i.product_id == pid).count());
    let n = files.len();
    let (added, error) =
        upload_files(calc, &pid, &color, files, existing, &|i| format!("Uploading {} of {n}…", i + 1)).await;
    let count = added.len();
    calc.images.update(|imgs| imgs.extend(added));
    calc.upload.set(Some(match error {
        Some(e) => UploadStatus { color, error: Some(e), ..Default::default() },
        None => UploadStatus { color, ok: Some(format!("{count} added")), ..Default::default() },
    }));
}

/// After a save: remember each colour's product and upload the queued photos.
async fn after_save(calc: Calc, ids: BTreeMap<String, String>, product_id: &str) -> (bool, String) {
    calc.variant_ids.set(ids.clone());
    let pending = calc.pending.get_untracked();
    if product_id.is_empty() || pending.is_empty() {
        return (false, String::new());
    }
    let total = pending.len();
    let mut colors: Vec<String> = Vec::new();
    for p in &pending {
        if !colors.contains(&p.color) {
            colors.push(p.color.clone());
        }
    }
    let mut added: Vec<ProductImage> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let done = std::cell::Cell::new(0);
    for color in colors {
        let pid = ids.get(&color).cloned().unwrap_or_else(|| product_id.to_string());
        let files: Vec<web_sys::File> = pending.iter().filter(|p| p.color == color).map(|p| p.file.clone()).collect();
        let existing =
            calc.images.with_untracked(|imgs| imgs.iter().chain(added.iter()).filter(|i| i.product_id == pid).count());
        let (got, error) = upload_files(calc, &pid, &color, files, existing, &|_| {
            done.set(done.get() + 1);
            format!("Uploading photo {} of {total}…", done.get())
        })
        .await;
        added.extend(got);
        if let Some(e) = error {
            failed.push(format!("{color}: {e}"));
        }
    }
    for p in &pending {
        let _ = web_sys::Url::revoke_object_url(&p.url);
    }
    calc.pending.set(Vec::new());
    calc.upload.set(None);
    let n = added.len();
    calc.images.update(|imgs| imgs.extend(added));
    if failed.is_empty() {
        (false, format!(" {n} photo(s) uploaded."))
    } else {
        (true, format!(" {n} photo(s) uploaded; failed — {}", failed.join("; ")))
    }
}

// ------------------------------------------------------------- details card

#[component]
fn DetailsCard(calc: Calc) -> impl IntoView {
    let image_url = calc.initial.with_value(|c| c.as_ref().and_then(|c| c.product.image_url.clone()));
    let siblings = calc.siblings.get_value();
    let categories = calc.categories.get_value();
    view! {
        <div class="card p-5">
            <div class="flex flex-wrap items-center justify-between gap-4 border-b border-(--color-border) pb-4">
                <div class="flex items-center gap-3">
                    {image_url.map(|url| {
                        let thumb = lattice_core::images::thumb_url(&url);
                        view! {
                            <div class="relative h-12 w-12 shrink-0 overflow-hidden rounded-lg border border-slate-200 bg-slate-100 shadow-xs">
                                <crate::kit::FallbackImg src=thumb fallback=url alt="Product photo" class="h-full w-full object-cover" />
                            </div>
                        }
                    })}
                    <div>
                        <h2 class="text-lg font-bold text-(--color-ink)">"Product & Costing Specification"</h2>
                        <p class="text-sm text-(--color-muted)">
                            {if calc.editing {
                                "Each colour is its own product with its own costing. Changes here save to this colour only."
                            } else {
                                "Code format CATEGORY/0001/COLOUR (e.g. LC/0001/WL). Tick several colours to create one product per colour."
                            }}
                        </p>
                    </div>
                </div>
                {(!siblings.is_empty()).then(|| view! {
                    <div class="flex flex-wrap items-center gap-1.5 text-xs">
                        <span class="font-semibold text-(--color-muted)">"Other colours:"</span>
                        {siblings
                            .into_iter()
                            .map(|s| {
                                let title = format!("Edit {} (a separate product)", s.color);
                                view! {
                                <A
                                    href=costing_href(&s.code)
                                    attr:class="rounded-full bg-(--color-sheet) px-2.5 py-1 font-medium hover:bg-slate-200"
                                    attr:title=title
                                >
                                    {s.color.clone()}
                                    " "
                                    <span class="font-mono">{format_money(Some(s.sp))}</span>
                                </A>
                            }})
                            .collect_view()}
                    </div>
                })}
            </div>

            <div class="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <CodeField calc=calc />
                <div>
                    <label class="label" for="calc-category">"Product Master Category"</label>
                    <select
                        id="calc-category"
                        class="select mt-1"
                        disabled=calc.editing
                        title=calc.editing.then_some("Part of the product code, so fixed once created")
                        on:change=move |e| calc.change_category(event_target_value(&e))
                    >
                        <option value="" selected=move || calc.category_id.get().is_empty()>"No category"</option>
                        {categories
                            .into_iter()
                            .map(|c| {
                                let id = c.id.clone();
                                let label = format!("{}{}", c.code.as_ref().filter(|s| !s.is_empty()).map(|s| format!("[{s}] ")).unwrap_or_default(), c.name);
                                view! { <option value=c.id selected=move || calc.category_id.get() == id>{label}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
                <div>
                    <label class="label" for="calc-name">"Product Name *"</label>
                    <input id="calc-name" class="input mt-1" placeholder="e.g. Lotus Glow (Pair)" required bind:value=calc.name />
                </div>
                <div>
                    <label class="label" for="calc-source">"Product Origin"</label>
                    <select id="calc-source" class="select mt-1" on:change=move |e| calc.change_origin(event_target_value(&e))>
                        {ORIGINS.map(|o| view! { <option value=o selected=move || calc.source.get() == o>{o}</option> }).collect_view()}
                    </select>
                    <p class="mt-1 text-[11px] text-(--color-muted)">
                        {move || match calc.source.get().as_str() {
                            "In-house" => "In-house manufacturing stages and bought-out items apply.",
                            "Outsource" => "Manufacturing stages start collapsed — type each item under Bought Out Items; new ones are saved to the Rates page.",
                            _ => "In-house stages plus bought-out items with their own markup.",
                        }}
                    </p>
                </div>
            </div>

            <div class="mt-4 border-t border-(--color-border) pt-4">
                <div class="flex flex-wrap items-center justify-between gap-3">
                    <span class="text-xs font-semibold text-(--color-ink)">{if calc.editing { "Colour & photos" } else { "Colours to create & photos" }}</span>
                    <label class="flex cursor-pointer items-center gap-2 text-sm font-medium text-(--color-ink)">
                        <input type="checkbox" class="h-4 w-4 rounded" bind:checked=calc.is_active />
                        <span>"Active in Product Master & Hamper Builder"</span>
                    </label>
                </div>
                <Show when=move || calc.product_id.get().is_empty()>
                    <p class="mt-1 text-xs text-(--color-muted)">"Photos you add now upload automatically when you save the product."</p>
                </Show>
                <ColorRows calc=calc />
            </div>
        </div>
    }
}

/// Product code (set from the category and colour), and every per-colour code it will save as.
#[component]
fn CodeField(calc: Calc) -> impl IntoView {
    let palette = calc.palette.get_value();
    view! {
        <div>
            <label class="label" for="calc-code">"Product Code *"</label>
            <input
                id="calc-code"
                class="input mt-1 font-mono font-semibold uppercase"
                placeholder="Choose a category"
                required
                readonly=calc.editing
                title=calc.editing.then_some("Fixed once created. Change the colour on the product page.")
                prop:value=move || calc.code.get()
                on:input=move |e| calc.code.set(event_target_value(&e).to_uppercase())
            />
            {move || {
                let (code, colors) = (calc.code.get(), calc.colors.get());
                if colors.len() > 1 && !code.trim().is_empty() {
                    view! {
                        <div class="mt-1.5 flex flex-wrap gap-1" title="One product is saved per color">
                            {codes_for_colors(&code, &colors)
                                .into_iter()
                                .map(|c| {
                                    let hex = palette.iter().find(|p| p.name == c.color).map(|p| p.hex.clone()).unwrap_or_default();
                                    view! {
                                        <span class="flex items-center gap-1 rounded border border-emerald-200 bg-emerald-50 px-1.5 py-0.5 font-mono text-[11px] font-semibold text-emerald-800">
                                            <span class="h-2 w-2 rounded-full border border-black/20" style=format!("background-color: {hex}")></span>
                                            {c.code}
                                        </span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                } else {
                    let p = calc.parsed_code();
                    let text = if calc.fetching_code.get() {
                        "Finding the next number…".to_string()
                    } else if p.is_valid {
                        format!("✓ {}/{}/{} ({})", p.category_code, p.serial, p.color_code, p.color_name)
                    } else {
                        "Set automatically from the category and colour".to_string()
                    };
                    view! {
                        <div class="mt-1 flex items-center justify-between text-xs">
                            <span class=if p.is_valid { "font-medium text-emerald-700" } else { "text-(--color-muted)" }>{text}</span>
                        </div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

fn files_of(e: &web_sys::Event) -> Vec<web_sys::File> {
    let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(list) = input.files() {
        for i in 0..list.length() {
            if let Some(f) = list.get(i) {
                out.push(f);
            }
        }
    }
    input.set_value("");
    out
}

/// New product: a row per colour, tick the ones to create. Editing: this
/// product's colour, plus any new ones added (each becomes a new product).
#[component]
fn ColorRows(calc: Calc) -> impl IntoView {
    let palette = calc.palette.get_value();
    let rows = {
        let palette = palette.clone();
        move || {
            if calc.editing {
                calc.colors
                    .get()
                    .iter()
                    .filter_map(|n| palette.iter().find(|c| &c.name == n).cloned())
                    .collect::<Vec<_>>()
            } else {
                palette.clone()
            }
        }
    };
    let addable = move || {
        let (sel, sib) = (calc.colors.get(), calc.sibling_colors.get_value());
        palette.iter().filter(|c| !sel.contains(&c.name) && !sib.contains(&c.name)).cloned().collect::<Vec<_>>()
    };
    view! {
        <ul class="mt-2 divide-y divide-(--color-border) rounded-lg border border-(--color-border)">
            {move || rows().into_iter().map(|col| view! { <ColorRow calc=calc col=col /> }).collect_view()}
        </ul>
        {move || {
            let add = addable();
            (calc.editing && !add.is_empty()).then(|| view! {
                <div class="mt-2 flex flex-wrap items-center gap-2 text-xs">
                    <label for="add-color" class="font-semibold text-(--color-ink)">"+ Add another colour"</label>
                    <select
                        id="add-color"
                        class="select w-auto py-1 text-xs"
                        on:change=move |e| {
                            let v = event_target_value(&e);
                            if !v.is_empty() {
                                calc.toggle_color(&v);
                            }
                        }
                    >
                        <option value="" selected=true>"Choose…"</option>
                        {add.into_iter().map(|c| view! { <option value=c.name.clone()>{format!("{} ({})", c.name, c.code)}</option> }).collect_view()}
                    </select>
                    <span class="text-(--color-muted)">"Saved as a new product, starting from this costing."</span>
                </div>
            })
        }}
    }
}

#[component]
fn ColorRow(calc: Calc, col: ProductColor) -> impl IntoView {
    let (name, code) = (col.name.clone(), col.code.clone());
    let own = calc.own_color.get_value() == name;
    let n1 = name.clone();
    let images = move || {
        calc.images.with(|imgs| {
            imgs.iter()
                .filter(|i| i.color.as_deref() == Some(&n1) || i.color_code.as_deref() == Some(&code))
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let n2 = name.clone();
    let pending = move || calc.pending.with(|p| p.iter().filter(|p| p.color == n2).cloned().collect::<Vec<_>>());
    let n3 = name.clone();
    let status = move || calc.upload.get().filter(|u| u.color == n3);
    let n4 = name.clone();
    let checked = move || calc.colors.with(|c| c.contains(&n4));
    let (n5, n6, n7) = (name.clone(), name.clone(), name.clone());
    let product_name = move || Some(calc.name.get()).filter(|n| !n.is_empty()).unwrap_or_else(|| "Product".into());
    let busy = move || calc.upload.with(|u| u.as_ref().is_some_and(|u| u.busy));
    view! {
        <li class="flex flex-wrap items-center gap-3 px-3 py-2">
            <label class="flex min-w-[190px] cursor-pointer items-center gap-2 text-xs font-medium text-(--color-ink)">
                {(!calc.editing).then(|| view! {
                    <input type="checkbox" class="h-4 w-4 rounded" prop:checked=checked on:change=move |_| calc.toggle_color(&n5) />
                })}
                <span class="inline-block h-3.5 w-3.5 rounded-full border border-black/20" style=format!("background-color: {}", col.hex)></span>
                {format!("{} ({})", col.name, col.code)}
                {calc.editing.then(|| if own {
                    view! { <span class="badge">"This product"</span> }.into_any()
                } else {
                    view! { <span class="text-[11px] font-normal text-amber-700">"New · created on save"</span> }.into_any()
                })}
            </label>
            <div class="flex flex-1 items-center gap-1.5">
                {move || {
                    let (imgs, pend) = (images(), pending());
                    let more = imgs.len().saturating_sub(5);
                    view! {
                        {(imgs.is_empty() && pend.is_empty()).then(|| view! { <span class="text-[11px] text-(--color-muted)">"No photos"</span> })}
                        {imgs.into_iter().take(5).map(|img| view! {
                            <ImagePreview src=img.url alt=format!("{} — {}", product_name(), n6) class="h-10 w-10 rounded-md border border-slate-200 bg-slate-100" />
                        }).collect_view()}
                        {pend.into_iter().map(|p| {
                            let key = p.key;
                            view! {
                                <span title="Uploads when you save" class="relative h-10 w-10 overflow-hidden rounded-md border border-dashed border-amber-400">
                                    <img src=p.url.clone() alt=p.file.name() class="h-full w-full object-cover opacity-80" />
                                    <button
                                        type="button"
                                        title="Remove"
                                        class="absolute top-0 right-0 flex h-4 w-4 items-center justify-center rounded-bl bg-white/90 text-[10px] leading-none text-red-600"
                                        on:click=move |_| calc.pending.update(|ps| {
                                            if let Some(gone) = ps.iter().find(|x| x.key == key) {
                                                let _ = web_sys::Url::revoke_object_url(&gone.url);
                                            }
                                            ps.retain(|x| x.key != key);
                                        })
                                    >
                                        "×"
                                    </button>
                                </span>
                            }
                        }).collect_view()}
                        {(more > 0).then(|| view! { <span class="text-[11px] text-(--color-muted)">{format!("+{more}")}</span> })}
                    }
                }}
            </div>
            {move || status().map(|s| view! {
                {s.busy.then(|| view! { <span class="text-[11px] text-(--color-muted)">"Uploading…"</span> })}
                {s.error.map(|e| view! { <span class="text-[11px] font-medium text-red-600">{e}</span> })}
                {s.ok.map(|o| view! { <span class="text-[11px] font-medium text-emerald-700">{o}</span> })}
            })}
            <label
                title=move || if calc.product_id.get().is_empty() { format!("Add {} photos; they upload when you save", col.name) } else { format!("Upload {} photos", col.name) }
                class=move || format!(
                    "rounded-md border px-2.5 py-1 text-[11px] font-semibold transition-colors {}",
                    if busy() { "cursor-not-allowed border-slate-200 text-slate-400" } else { "cursor-pointer border-(--color-brand) text-(--color-brand) hover:bg-emerald-50" }
                )
            >
                "📷 Upload"
                <input
                    type="file"
                    accept="image/*"
                    multiple
                    hidden
                    disabled=busy
                    on:change=move |e| {
                        let files = files_of(&e);
                        if files.is_empty() {
                            return;
                        }
                        // Uploading to a colour also selects it.
                        if !calc.colors.with_untracked(|c| c.contains(&n7)) {
                            calc.toggle_color(&n7);
                        }
                        let color = n7.clone();
                        spawn_local(upload_for_color(calc, color, files));
                    }
                />
            </label>
            {(calc.editing && !own).then(|| {
                let n = name.clone();
                view! {
                    <button type="button" class="text-[11px] font-semibold text-red-700 hover:underline" on:click=move |_| calc.toggle_color(&n)>
                        "Remove"
                    </button>
                }
            })}
        </li>
    }
}

// ----------------------------------------------------------------- gallery

fn is_of_color(img: &ProductImage, c: &ProductColor) -> bool {
    img.color.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(&c.name))
        || img.color_code.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(&c.code))
}

fn is_general(img: &ProductImage) -> bool {
    img.color.as_deref().is_none_or(str::is_empty) && img.color_code.as_deref().is_none_or(str::is_empty)
}

/// Set the cover, re-tag a photo's colour or move it to the Bin.
#[component]
fn Gallery(calc: Calc) -> impl IntoView {
    let tab = RwSignal::new("all".to_string());
    let error = RwSignal::new(None::<String>);
    let success = RwSignal::new(None::<String>);
    let palette = calc.palette.get_value();
    let colors = {
        let palette = palette.clone();
        move || {
            let sel = calc.colors.get();
            if sel.is_empty() {
                palette.clone()
            } else {
                palette.iter().filter(|c| sel.contains(&c.name)).cloned().collect()
            }
        }
    };
    let tab_class = move |active: bool| {
        format!(
            "rounded-md px-3 py-1.5 text-xs font-semibold transition-all {}",
            if active { "bg-(--color-brand) text-white shadow-xs" } else { "text-(--color-muted) hover:bg-slate-100" }
        )
    };
    let owner_of = move |id: &str| {
        calc.images
            .with_untracked(|imgs| imgs.iter().find(|i| i.id == id).map(|i| i.product_id.clone()))
            .unwrap_or_else(|| calc.product_id.get_untracked())
    };
    // A new cover takes the badge from the old one of the same product.
    let with_primary = move |img: ProductImage| {
        calc.images.update(|list| {
            for i in list.iter_mut().filter(|i| i.product_id == img.product_id) {
                i.is_primary = i.id == img.id;
            }
        });
    };
    let filtered = move || {
        let t = tab.get().to_lowercase();
        calc.images
            .get()
            .into_iter()
            .filter(|img| match t.as_str() {
                "all" => true,
                "general" => is_general(img),
                t => {
                    img.color.as_deref().is_some_and(|c| c.to_lowercase() == t)
                        || img.color_code.as_deref().is_some_and(|c| c.to_lowercase() == t)
                }
            })
            .collect::<Vec<_>>()
    };
    let colors2 = colors.clone();
    view! {
        <div class="space-y-4">
            <div class="flex flex-wrap items-center justify-between gap-3 border-b border-(--color-border) pb-3">
                <div>
                    <h3 class="text-sm font-bold text-(--color-ink)">"Product Images & Color Variations"</h3>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Set the cover, change a photo's color or remove it. Add photos in the color rows above."</p>
                </div>
            </div>
            <div class="flex flex-wrap items-center gap-1 border-b border-slate-200 pb-2">
                <button type="button" class=move || tab_class(tab.get() == "all") on:click=move |_| tab.set("all".into())>
                    {move || format!("All Photos ({})", calc.images.with(Vec::len))}
                </button>
                {move || colors().into_iter().map(|c| {
                    let (name, n2) = (c.name.clone(), c.name.clone());
                    let count = calc.images.with(|imgs| imgs.iter().filter(|i| is_of_color(i, &c)).count());
                    view! {
                        <button type="button" class=move || format!("flex items-center gap-1.5 {}", tab_class(tab.get().to_lowercase() == name.to_lowercase())) on:click=move |_| tab.set(n2.clone())>
                            <span class="h-2 w-2 rounded-full border border-black/20" style=format!("background-color: {}", c.hex)></span>
                            {format!("{} ({count})", c.name)}
                        </button>
                    }
                }).collect_view()}
                {move || {
                    let general = calc.images.with(|imgs| imgs.iter().filter(|i| is_general(i)).count());
                    (general > 0).then(|| view! {
                        <button type="button" class=move || tab_class(tab.get() == "general") on:click=move |_| tab.set("general".into())>{format!("General ({general})")}</button>
                    })
                }}
            </div>
            {move || error.get().map(|e| view! { <div class="rounded-lg border border-red-200 bg-red-50 p-2.5 text-xs font-medium text-red-700">{e}</div> })}
            {move || success.get().map(|s| view! { <div class="rounded-lg border border-emerald-200 bg-emerald-50 p-2.5 text-xs font-medium text-emerald-800">{s}</div> })}
            {move || {
                let list = filtered();
                if list.is_empty() {
                    let what = if tab.get() == "all" { "this product".to_string() } else { format!("{} finish", tab.get()) };
                    return view! { <div class="rounded-lg border border-dashed border-slate-200 p-8 text-center text-xs text-(--color-muted)">{format!("No images uploaded for {what} yet.")}</div> }.into_any();
                }
                let cols = colors2();
                view! {
                    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
                        {list.into_iter().map(|img| {
                            let tag = cols.iter().find(|c| is_of_color(&img, c)).cloned();
                            let (id1, id2, id3) = (img.id.clone(), img.id.clone(), img.id.clone());
                            view! {
                                <ImageCard
                                    img=img
                                    colors=cols.clone()
                                    tag=tag
                                    on_delete=move || {
                                        if !confirm("Move this image to the Bin? It will stay in the Bin for 30 days and can be restored anytime.") {
                                            return;
                                        }
                                        let (id, owner) = (id1.clone(), owner_of(&id1));
                                        spawn_local(async move {
                                            match photos::delete_image(&id, &owner).await {
                                                Ok(promoted) => {
                                                    calc.images.update(|l| l.retain(|i| i.id != id));
                                                    if let Some(p) = promoted { with_primary(p) }
                                                    success.set(Some("Image moved to Bin. You can restore it from the Bin within 30 days.".into()));
                                                }
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                        });
                                    }
                                    on_color=move |c: String| {
                                        let id = id2.clone();
                                        spawn_local(async move {
                                            let raw = (c != "General").then_some(c);
                                            match photos::change_color(&id, raw.as_deref()).await {
                                                Ok(img) => calc.images.update(|l| if let Some(x) = l.iter_mut().find(|i| i.id == id) { *x = img }),
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                        });
                                    }
                                    on_primary=move || {
                                        let (id, owner) = (id3.clone(), owner_of(&id3));
                                        spawn_local(async move {
                                            match photos::set_primary(&id, &owner).await {
                                                Ok(img) => {
                                                    with_primary(img);
                                                    success.set(Some("Primary cover image updated.".into()));
                                                }
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                        });
                                    }
                                />
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn ImageCard(
    img: ProductImage,
    colors: Vec<ProductColor>,
    tag: Option<ProductColor>,
    on_delete: impl Fn() + 'static,
    on_color: impl Fn(String) + 'static,
    on_primary: impl Fn() + 'static,
) -> impl IntoView {
    let current = img.color.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| "General".into());
    let thumb = lattice_core::images::thumb_url(&img.url);
    let hex = tag.map_or_else(|| FALLBACK_COLOR_HEX.to_string(), |c| c.hex);
    view! {
        <div class=format!(
            "group relative overflow-hidden rounded-xl border bg-white shadow-xs transition-all {}",
            if img.is_primary { "border-(--color-brand) ring-2 ring-(--color-brand)/20" } else { "border-slate-200 hover:border-slate-300" }
        )>
            <div class="relative aspect-square w-full overflow-hidden bg-slate-100">
                <crate::kit::FallbackImg src=thumb fallback=img.url.clone() alt=img.caption.clone().unwrap_or_else(|| "Product Photo".into()) class="absolute inset-0 h-full w-full object-cover transition-transform duration-200 group-hover:scale-105" />
                {img.is_primary.then(|| view! {
                    <span class="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-emerald-700/90 px-2 py-0.5 text-[10px] font-bold text-white shadow-xs">"★ Primary"</span>
                })}
                <button type="button" title="Delete photo" on:click=move |_| on_delete()
                    class="absolute top-2 right-2 flex h-6 w-6 items-center justify-center rounded-full bg-white/90 text-slate-500 opacity-0 shadow-sm transition-colors group-hover:opacity-100 hover:bg-red-50 hover:text-red-600">
                    "×"
                </button>
            </div>
            <div class="space-y-1.5 p-2">
                <div class="flex items-center justify-between gap-1">
                    <div class="flex min-w-0 items-center gap-1.5">
                        <span class="h-2 w-2 shrink-0 rounded-full border border-black/20" style=format!("background-color: {hex}")></span>
                        <select class="cursor-pointer truncate border-0 bg-transparent p-0 text-[11px] font-medium text-slate-700 focus:ring-0" on:change=move |e| on_color(event_target_value(&e))>
                            {colors.into_iter().map(|c| {
                                let sel = c.name == current;
                                view! { <option value=c.name.clone() selected=sel>{format!("{} ({})", c.name, c.code)}</option> }
                            }).collect_view()}
                            <option value="General" selected=current == "General">"General / All"</option>
                        </select>
                    </div>
                    {(!img.is_primary).then(|| view! {
                        <button type="button" title="Set as primary cover photo" on:click=move |_| on_primary()
                            class="text-[10px] font-medium text-slate-400 transition-colors hover:text-(--color-brand)">
                            "Make Primary"
                        </button>
                    })}
                </div>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------- stages

#[component]
fn StageSection(calc: Calc, stage: StageNode) -> impl IntoView {
    let code = stage.row.code.clone();
    let (c1, c2, c3, c4, c5) = (code.clone(), code.clone(), code.clone(), code.clone(), code.clone());
    let lines = Memo::new(move |_| {
        calc.lines.with(|ls| ls.iter().filter(|x| x.l.stage_code == c1).map(|x| x.key).collect::<Vec<_>>())
    });
    let lines_total = Memo::new(move |_| {
        dec(calc.lines.with(|ls| ls.iter().filter(|x| x.l.stage_code == c2).fold(0.0, |s, x| s + f(x.l.line_total))))
    });
    let stage_total = Memo::new(move |_| with_overhead(lines_total.get(), parse_num(&calc.overhead_for(&c3))));
    let collapsed = Memo::new(move |_| calc.collapsed.with(|c| c.contains(&c4)));
    let is_machine = code == "machine";
    let is_bought = code == BOUGHT_OUT;
    let has_cats = !stage.categories.is_empty();
    // Outsourced: bought-out items are typed in and saved to the master.
    let free_entry = move || is_bought && calc.source.get() == "Outsource";
    let name = stage.row.name.clone();
    let stage = StoredValue::new(stage);
    let hint = {
        let name = name.clone();
        move || {
            let n = lines.with(Vec::len);
            if collapsed.get() {
                return format!("(Collapsed — {n} line{}, click + to edit)", if n == 1 { "" } else { "s" });
            }
            let _ = &name;
            if is_machine {
                "(Per minute or hour of machine time, or by size — as set in the master)".into()
            } else if is_bought {
                if free_entry() {
                    "(Type Category → Subcategory → Variety, rate & unit — new items are added to the Rates page on save; + MRP for items with a printed MRP)".into()
                } else if has_cats {
                    "(Category → Subcategory → Variety — Cost Price = Rate × Qty; + MRP for items with a printed MRP)"
                        .into()
                } else {
                    "(Free-text description, rate & quantity — Cost Price = Rate × Qty; + MRP for items with a printed MRP)".into()
                }
            } else if has_cats {
                "(Category → Subcategory → Variety, dimensions & wastage)".into()
            } else {
                "(Free-text description, rate & quantity)".into()
            }
        }
    };
    let n2 = name.clone();
    let n3 = name.clone();
    let c6 = code.clone();
    view! {
        <div class="card overflow-hidden shadow-sm">
            <div class="flex flex-wrap items-center justify-between gap-2 border-b border-(--color-border) bg-(--color-sheet) px-4 py-3">
                <div class="flex items-center gap-3">
                    <button
                        type="button"
                        on:click=move |_| calc.toggle_stage(&c5)
                        aria-expanded=move || (!collapsed.get()).to_string()
                        class="flex h-6 w-6 items-center justify-center rounded border border-(--color-border) bg-white text-sm font-bold leading-none text-(--color-brand-dark) shadow-xs transition-colors hover:bg-emerald-50"
                    >
                        {move || if collapsed.get() { "+" } else { "−" }}
                    </button>
                    <span class="flex h-7 w-7 items-center justify-center rounded-full bg-(--color-brand) text-xs font-bold text-white">{stage.with_value(|s| s.row.sort_order)}</span>
                    <h3 class="text-base font-semibold text-(--color-ink)">{name.clone()}</h3>
                    <span class="text-xs text-(--color-muted)">{hint}</span>
                </div>
                <div class="flex items-center gap-3">
                    <label class="flex items-center gap-1.5 text-xs text-(--color-muted)" title=format!("Overhead added to the {n2} subtotal")>
                        "Overhead"
                        <span class="flex items-center rounded-lg border border-(--color-border) bg-white pr-1.5 shadow-xs">
                            <input
                                type="number"
                                min="0"
                                step="any"
                                placeholder="0"
                                aria-label=format!("{n3} overhead percent")
                                class="w-14 rounded-lg bg-transparent px-1.5 py-1 text-right font-mono text-xs text-(--color-ink) outline-none"
                                prop:value=move || calc.overhead_for(&c6)
                                on:input={
                                    let code = code.clone();
                                    move |e| calc.set_overhead(&code, event_target_value(&e))
                                }
                            />
                            "%"
                        </span>
                        {move || {
                            let added = dec(f(stage_total.get()) - f(lines_total.get()));
                            (!parse_num(&calc.overhead_for(&code)).is_zero()).then(|| view! { <span class="font-mono text-(--color-brand-dark)">{format!("+{}", format_money(Some(added)))}</span> })
                        }}
                    </label>
                    <span class="text-xs text-(--color-muted)">"Stage Total:"</span>
                    <span class="rounded-lg bg-white px-2.5 py-1 font-mono text-sm font-bold text-(--color-brand-dark) shadow-sm">{move || format_money(Some(stage_total.get()))}</span>
                </div>
            </div>
            <Show when=move || !collapsed.get()>
                <div class="overflow-x-auto p-2">
                    <table class="w-full text-left text-xs">
                        <thead>
                            <tr class="border-b border-(--color-border) font-semibold text-(--color-muted)">
                                {move || if has_cats || free_entry() {
                                    view! {
                                        <th class="min-w-32.5 p-2">"Category"</th>
                                        <th class="min-w-32.5 p-2">"Subcategory"</th>
                                        <th class="min-w-30 p-2">"Variety"</th>
                                    }.into_any()
                                } else {
                                    view! { <th class="min-w-65 p-2">"Description"</th> }.into_any()
                                }}
                                <th class="min-w-21.25 p-2">"Length"</th>
                                <th class="min-w-21.25 p-2">"Breadth"</th>
                                <th class="min-w-20 p-2">"Dim Unit"</th>
                                {is_machine.then(|| view! { <th class="min-w-27.5 p-2">"Duration (Mins)"</th> })}
                                {move || if has_cats && !free_entry() {
                                    view! { <th class="min-w-27.5 p-2">"Rate / Unit"</th> }.into_any()
                                } else {
                                    view! { <th class="min-w-21.25 p-2">"Rate"</th><th class="min-w-22.5 p-2">"Unit"</th> }.into_any()
                                }}
                                <th class="min-w-16.25 p-2">"Qty"</th>
                                <th class="min-w-18.75 p-2" title=is_bought.then_some("Printed MRP per piece, GST included")>{if is_bought { "MRP" } else { "Wastage %" }}</th>
                                <th class="min-w-23.75 p-2 text-right">"Total Cost"</th>
                                <th class="min-w-25 p-2 text-center">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-(--color-border)">
                            <Show when=move || lines.with(Vec::is_empty)>
                                <tr><td colspan="13" class="py-6 text-center text-sm text-(--color-muted)">{format!("No lines added for {}. Click “+ Add Line” below to start.", stage.with_value(|s| s.row.name.clone()))}</td></tr>
                            </Show>
                            <For each=move || lines.get() key=|k| *k let:key>
                                <LineRow calc=calc stage=stage key=key free_entry=Signal::derive(free_entry) />
                            </For>
                        </tbody>
                    </table>
                </div>
                <div class="border-t border-(--color-border) bg-slate-50/50 p-2 text-right">
                    <button type="button" class="btn-secondary px-3 py-1 text-xs" on:click=move |_| stage.with_value(|s| calc.add_to_stage(&s.row.code))>
                        {format!("+ Add {} Line", stage.with_value(|s| s.row.name.clone()))}
                    </button>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn LineRow(calc: Calc, stage: StoredValue<StageNode>, key: u64, free_entry: Signal<bool>) -> impl IntoView {
    let line = Memo::new(move |_| {
        calc.lines.with(|ls| ls.iter().find(|x| x.key == key).map(|x| x.l.clone())).unwrap_or_default()
    });
    let code = stage.with_value(|s| s.row.code.clone());
    let is_machine = code == "machine";
    let is_bought = code == BOUGHT_OUT;
    let has_cats = stage.with_value(|s| !s.categories.is_empty());
    let from_master = move || has_cats && !free_entry.get();
    let timed = move || {
        is_machine
            && is_time_unit(Some(&Some(line.get().unit).filter(|u| !u.is_empty()).unwrap_or_else(|| "min".into())))
    };
    // Length and breadth only count for rates per length or area, not per piece.
    let sized = move || !timed() && is_sized_unit(Some(&line.get().unit));
    view! {
        <tr class="transition-colors hover:bg-slate-50/75">
            {move || if free_entry.get() {
                view! { <TypedCells calc=calc stage=stage key=key line=line /> }.into_any()
            } else if has_cats {
                view! { <PickCells calc=calc stage=stage key=key line=line /> }.into_any()
            } else {
                view! {
                    <td class="p-2">
                        <input
                            class="input px-2 py-1 text-xs"
                            placeholder=if is_bought { "e.g. Water bottle, ceramic jar, chocolates" } else { "e.g. Courier packaging, ribbon, gift tag" }
                            prop:value=move || line.get().item_name
                            on:input=move |e| { let v = event_target_value(&e); calc.update(key, |l| l.item_name = v) }
                        />
                    </td>
                }.into_any()
            }}
            {move || if sized() {
                view! {
                    <td class="p-2"><NumInput value=Signal::derive(move || line.get().length) nullable=true placeholder=if is_bought { "—" } else { "12" } on_change=move |v| calc.update(key, |l| l.length = v) /></td>
                    <td class="p-2"><NumInput value=Signal::derive(move || line.get().breadth) nullable=true placeholder=if is_bought { "—" } else { "12" } on_change=move |v| calc.update(key, |l| l.breadth = v) /></td>
                    <td class="p-2">
                        <select class="select px-2 py-1 text-xs" on:change=move |e| { let v = event_target_value(&e); calc.update(key, |l| l.dimension_unit = Some(v)) }>
                            {[("inch", "Inch (\")"), ("mm", "mm"), ("cm", "cm")].map(|(v, label)| view! {
                                <option value=v selected=move || line.get().dimension_unit.as_deref().unwrap_or("inch") == v>{label}</option>
                            }).collect_view()}
                        </select>
                    </td>
                }.into_any()
            } else {
                view! {
                    <td colspan="3" class="p-2 text-center text-[11px] text-(--color-muted)">
                        {move || if timed() { "Priced on time".to_string() } else { format!("Priced per {}", line.get().unit) }}
                    </td>
                }.into_any()
            }}
            {is_machine.then(|| view! {
                {move || if timed() {
                    view! {
                        <td class="p-2">
                            <div class="flex items-center gap-1">
                                <NumInput value=Signal::derive(move || line.get().duration_minutes) nullable=true placeholder="15" on_change=move |v| calc.update(key, |l| l.duration_minutes = v) />
                                <span class="text-[11px] text-(--color-muted)">"min"</span>
                            </div>
                        </td>
                    }.into_any()
                } else {
                    view! { <td class="p-2 text-center text-[11px] text-(--color-muted)">"—"</td> }.into_any()
                }}
            })}
            {move || if from_master() {
                view! {
                    <td class="whitespace-nowrap p-2 font-mono text-xs font-semibold" title="Set on the Rates page">
                        {move || { let l = line.get(); if l.variety_name.as_deref().is_some_and(|v| !v.is_empty()) || !l.rate.is_zero() { format!("{}/{}", format_money(Some(l.rate)), l.unit) } else { "—".into() } }}
                    </td>
                }.into_any()
            } else {
                view! {
                    <td class="p-2"><NumInput value=Signal::derive(move || Some(line.get().rate)) class="font-mono" on_change=move |v| calc.update(key, |l| l.rate = v.unwrap_or_default()) /></td>
                    <td class="p-2"><UnitCell value=Signal::derive(move || line.get().unit) on_change=move |u| calc.update(key, |l| l.unit = u) /></td>
                }.into_any()
            }}
            <td class="p-2"><NumInput value=Signal::derive(move || line.get().qty) min="0" on_change=move |v| calc.update(key, |l| l.qty = Some(v.unwrap_or_default())) /></td>
            {if is_bought {
                view! { <MrpCell calc=calc key=key line=line /> }.into_any()
            } else {
                view! {
                    {move || if from_master() {
                        view! {
                            <td class="p-2 text-center font-mono text-xs font-semibold" title="Set on the Rates page">
                                {move || { let l = line.get(); if l.variety_name.as_deref().is_some_and(|v| !v.is_empty()) { format!("{}%", num_text(l.wastage_pct)) } else { "—".into() } }}
                            </td>
                        }.into_any()
                    } else {
                        view! { <td class="p-2"><NumInput value=Signal::derive(move || Some(line.get().wastage_pct)) min="0" on_change=move |v| calc.update(key, |l| l.wastage_pct = v.unwrap_or_default()) /></td> }.into_any()
                    }}
                }.into_any()
            }}
            <td class="p-2 text-right font-mono font-semibold text-(--color-ink)">{move || format_money(Some(line.get().line_total))}</td>
            <td class="p-2 text-center">
                <div class="flex items-center justify-center gap-1">
                    <button type="button" title="Duplicate this line (copy details to quick-edit)" on:click=move |_| calc.duplicate(key)
                        class="rounded px-2 py-1 text-[11px] font-semibold text-(--color-brand) transition-colors hover:bg-emerald-50 hover:text-(--color-brand-dark)">
                        "Duplicate"
                    </button>
                    <button type="button" title="Remove line" on:click=move |_| calc.remove(key)
                        class="rounded px-1.5 py-1 text-[11px] font-semibold text-red-600 transition-colors hover:bg-red-50">"×"</button>
                </div>
            </td>
        </tr>
    }
}

/// Units a priced-by-size line can use (minutes and hours belong to the machine stage),
/// keeping an old unit that is no longer in the list.
#[component]
fn UnitCell(value: Signal<String>, on_change: impl Fn(String) + 'static) -> impl IntoView {
    view! {
        <select class="select px-2 py-1 text-xs" on:change=move |e| on_change(event_target_value(&e))>
            {COMMON_UNITS.iter().filter(|u| **u != "min" && **u != "hour").map(|u| view! {
                <option value=*u selected=move || value.get() == *u>{*u}</option>
            }).collect_view()}
            {move || { let v = value.get(); (!v.is_empty() && !COMMON_UNITS.contains(&v.as_str())).then(|| view! { <option value=v.clone() selected=true>{v.clone()}</option> }) }}
        </select>
    }
}

fn pick_options(opts: Vec<(String, String)>, current: String, placeholder: &'static str) -> impl IntoView {
    // A saved choice that has since left the master stays visible, flagged.
    let missing = !current.is_empty() && !opts.iter().any(|(_, n)| *n == current);
    view! {
        <option value="" selected=current.is_empty()>{placeholder}</option>
        {missing.then(|| view! { <option value=current.clone() selected=true>{format!("{current} (not in master)")}</option> })}
        {opts.into_iter().map(|(_, n)| { let sel = n == current; let label = n.clone(); view! { <option value=n selected=sel>{label}</option> } }).collect_view()}
    }
}

/// Category → Subcategory → Variety picked from the Rates page.
#[component]
fn PickCells(calc: Calc, stage: StoredValue<StageNode>, key: u64, line: Memo<ProductCostLine>) -> impl IntoView {
    let cats = move || {
        stage.with_value(|s| s.categories.iter().map(|c| (c.row.id.clone(), c.row.name.clone())).collect::<Vec<_>>())
    };
    let subs = move || {
        let l = line.get();
        stage.with_value(|s| {
            s.categories
                .iter()
                .find(|c| c.row.name == l.category_name)
                .map(|c| c.subcategories.iter().map(|x| (x.row.id.clone(), x.row.name.clone())).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    };
    let vars = move || {
        let l = line.get();
        stage.with_value(|s| {
            s.categories
                .iter()
                .find(|c| c.row.name == l.category_name)
                .and_then(|c| c.subcategories.iter().find(|x| Some(&x.row.name) == l.subcategory_name.as_ref()))
                .map(|x| x.varieties.iter().map(|v| (v.id.clone(), v.name.clone())).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    };
    let missing_class = |opts: &[(String, String)], v: &str| {
        if !v.is_empty() && !opts.iter().any(|(_, n)| n == v) { "border-amber-400 text-amber-800" } else { "" }
    };
    view! {
        <td class="p-2">
            {move || { let (o, v) = (cats(), line.get().category_name); let cls = format!("select px-2 py-1 text-xs {}", missing_class(&o, &v)); view! {
                <select class=cls on:change=move |e| calc.set_category(key, event_target_value(&e))>
                    {pick_options(o, v, "Select category…")}
                </select>
            } }}
        </td>
        <td class="p-2">
            {move || { let (o, l) = (subs(), line.get()); let v = l.subcategory_name.clone().unwrap_or_default(); let cls = format!("select px-2 py-1 text-xs font-medium {}", missing_class(&o, &v)); view! {
                <select class=cls disabled=l.category_name.is_empty() on:change=move |e| calc.set_subcategory(key, event_target_value(&e))>
                    {pick_options(o, v, "Select…")}
                </select>
            } }}
        </td>
        <td class="p-2">
            {move || { let (o, l) = (vars(), line.get()); let v = l.variety_name.clone().unwrap_or_default(); let cls = format!("select px-2 py-1 text-xs font-semibold text-(--color-brand-dark) {}", missing_class(&o, &v)); view! {
                <select class=cls disabled=l.subcategory_name.as_deref().is_none_or(str::is_empty) on:change=move |e| calc.set_variety(key, event_target_value(&e))>
                    {pick_options(o, v, "Select…")}
                </select>
            } }}
        </td>
    }
}

fn by_name<'a, T>(rows: impl IntoIterator<Item = &'a T>, name: &str, of: impl Fn(&T) -> &str) -> Option<&'a T>
where
    T: 'a,
{
    rows.into_iter().find(|r| of(r).to_lowercase() == name.trim().to_lowercase())
}

/// Category → Subcategory → Variety typed in (outsourced products); existing
/// names are suggested, new ones go into the master on save.
#[component]
fn TypedCells(calc: Calc, stage: StoredValue<StageNode>, key: u64, line: Memo<ProductCostLine>) -> impl IntoView {
    let set = move |field: u8, value: String| {
        let mut next = line.get_untracked();
        match field {
            0 => next.category_name = value,
            1 => next.subcategory_name = Some(value),
            _ => next.variety_name = Some(value),
        }
        let found = stage.with_value(|s| {
            by_name(&s.categories, &next.category_name, |c| &c.row.name)
                .and_then(|c| {
                    by_name(&c.subcategories, next.subcategory_name.as_deref().unwrap_or_default(), |x| &x.row.name)
                })
                .and_then(|x| by_name(&x.varieties, next.variety_name.as_deref().unwrap_or_default(), |v| &v.name))
                .cloned()
        });
        calc.update(key, |l| {
            l.category_name.clone_from(&next.category_name);
            l.subcategory_name.clone_from(&next.subcategory_name);
            l.variety_name.clone_from(&next.variety_name);
            l.item_name = [next.subcategory_name.clone(), next.variety_name.clone()]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            l.cost_variety_id = found.as_ref().map(|v| v.id.clone());
            // Picking an existing variety brings its master rate and unit.
            if let (Some(v), 2) = (&found, field) {
                l.rate = v.default_rate;
                l.unit.clone_from(&v.unit);
            }
        });
    };
    let options = move |field: u8| -> Vec<String> {
        let l = line.get();
        stage.with_value(|s| match field {
            0 => s.categories.iter().map(|c| c.row.name.clone()).collect(),
            1 => by_name(&s.categories, &l.category_name, |c| &c.row.name)
                .map(|c| c.subcategories.iter().map(|x| x.row.name.clone()).collect())
                .unwrap_or_default(),
            _ => by_name(&s.categories, &l.category_name, |c| &c.row.name)
                .and_then(|c| {
                    by_name(&c.subcategories, l.subcategory_name.as_deref().unwrap_or_default(), |x| &x.row.name)
                })
                .map(|x| x.varieties.iter().map(|v| v.name.clone()).collect())
                .unwrap_or_default(),
        })
    };
    [(0u8, "Category", "e.g. Drinkware"), (1, "Subcategory", "e.g. Steel Bottle"), (2, "Variety", "e.g. 500ml")]
        .map(|(field, label, placeholder)| {
            let list_id = format!("{key}-{field}");
            let value = move || {
                let l = line.get();
                match field {
                    0 => l.category_name,
                    1 => l.subcategory_name.unwrap_or_default(),
                    _ => l.variety_name.unwrap_or_default(),
                }
            };
            view! {
                <td class="p-2">
                    <input class="input px-2 py-1 text-xs" list=list_id.clone() placeholder=placeholder aria-label=label prop:value=value on:input=move |e| set(field, event_target_value(&e)) />
                    <datalist id=list_id>{move || options(field).into_iter().map(|o| view! { <option value=o></option> }).collect_view()}</datalist>
                </td>
            }
        })
        .collect_view()
}

/// A bought-out item's printed MRP (per piece, GST included): "+ MRP" until
/// one is typed, then the MRP with the margin it leaves at that price.
#[component]
fn MrpCell(calc: Calc, key: u64, line: Memo<ProductCostLine>) -> impl IntoView {
    let open = RwSignal::new(false);
    let gst = calc.gst;
    view! {
        {move || if !open.get() && line.get().mrp.is_none() {
            view! {
                <td class="p-2 text-center">
                    <button type="button" title="This item has an MRP printed on it" on:click=move |_| open.set(true)
                        class="rounded border border-(--color-border) px-2 py-1 text-[11px] font-semibold text-(--color-brand) transition-colors hover:bg-emerald-50">
                        "+ MRP"
                    </button>
                </td>
            }.into_any()
        } else {
            view! {
                <td class="p-2" on:focusout=move |_| open.set(false)>
                    <NumInput value=Signal::derive(move || line.get().mrp) nullable=true placeholder="MRP" label="MRP per piece, GST included" class="font-mono"
                        on_change=move |v| calc.update(key, |l| l.mrp = v.filter(|m| *m > Decimal::ZERO)) />
                    {move || { let l = line.get(); mrp_margin(&l, gst).map(|m| {
                        let max_sp = dec(f(l.mrp.unwrap_or_default()) / (1.0 + f(gst)));
                        view! {
                            <div class=format!("mt-0.5 whitespace-nowrap text-[10px] {}", if m > Decimal::ZERO { "text-emerald-700" } else { "font-semibold text-red-600" })>
                                {format!("Max SP {} · {} margin", format_money(Some(max_sp)), format_pct(Some(m), 0))}
                            </div>
                        }
                    }) }}
                </td>
            }.into_any()
        }}
    }
}

// ----------------------------------------------------------------- summary

#[component]
fn Summary(calc: Calc, on_save: impl Fn() + 'static) -> impl IntoView {
    let caption = "text-xs font-semibold uppercase tracking-wider text-(--color-muted)";
    let note = "mt-1 text-[11px] text-(--color-muted)";
    view! {
        <div class="card border-2 border-(--color-brand)/20 bg-linear-to-br from-white to-slate-50 p-5 shadow-md">
            <h3 class="mb-4 text-base font-bold text-(--color-ink)">"Cost Breakdown & Selling Price Master"</h3>
            {move || view! { <CostBreakdown totals=calc.totals.get() /> }}
            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <div class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
                    <span class=caption>"Total Cost Price (CP)"</span>
                    <div class="mt-1 font-mono text-2xl font-black text-(--color-ink)">{move || format_money(Some(calc.totals.get().total_cost))}</div>
                    <p class=note>"Sum of Material + HW + Finishing + Machine + Misc + Bought Out"</p>
                </div>
                <div class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
                    <label for="calc-markup" class=caption>"Markup %"</label>
                    <div class="mt-1 flex items-center gap-1">
                        <input id="calc-markup" type="number" step="any" class="input input-num py-1 font-mono text-lg font-bold"
                            prop:value=move || calc.markup.get() on:input=move |e| calc.change_markup(event_target_value(&e)) />
                        <span class="text-base font-bold text-(--color-muted)">"%"</span>
                    </div>
                    <p class=note>"Selling Price = CP ÷ (1 − Markup%)"</p>
                    <Show when=move || { parse_num(&calc.markup.get()) >= Decimal::from(100) }>
                        <p class="mt-1 text-[11px] font-semibold text-red-600">"Markup must be under 100% — at 100% the formula has no finite price."</p>
                    </Show>
                </div>
                <div class=move || format!("rounded-xl border p-4 shadow-sm {}", if calc.over_mrp() { "border-red-500 bg-red-50/60" } else { "border-(--color-brand) bg-emerald-50/40" })>
                    <label for="calc-sp" class="text-xs font-semibold uppercase tracking-wider text-(--color-brand-dark)">"Selling Price (SP)"</label>
                    <div class="mt-1 flex items-center gap-1">
                        <span class="text-lg font-bold text-(--color-brand-dark)">"₹"</span>
                        <input id="calc-sp" type="number" step="any" class="input input-num py-1 font-mono text-lg font-black text-(--color-brand-dark)"
                            placeholder=move || num_text(calc.totals.get().calculated_sp)
                            prop:value=move || calc.manual_sp.get()
                            on:input=move |e| calc.change_sp(event_target_value(&e))
                            on:blur=move |_| calc.round_manual_sp() />
                    </div>
                    <p class=note>"Default catalogue price saved to product"</p>
                    {move || calc.mrp_max.get().map(|max| if calc.over_mrp() {
                        view! {
                            <div role="alert" class="mt-1 flex flex-wrap items-center gap-2 text-[11px] font-semibold text-red-700">
                                {format!("Above MRP: max SP is {}", format_money(Some(max)))}
                                <button type="button" class="rounded border border-red-300 bg-white px-2 py-0.5 text-red-700 transition-colors hover:bg-red-100"
                                    on:click=move |_| calc.change_sp(num_text(max))>
                                    {format!("Set to {}", format_money(Some(max)))}
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <p class=note>{format!("Max {} under MRP (incl. GST)", format_money(Some(max)))}</p> }.into_any()
                    })}
                </div>
                <div class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
                    <span class=caption>"Target Margin"</span>
                    <div class="mt-1 font-mono text-2xl font-black text-emerald-700">{move || format_pct(Some(calc.effective_margin()), 1)}</div>
                    <p class=note>"Margin = (SP − CP) / SP"</p>
                </div>
            </div>
            <div class="mt-4">
                <label class="label" for="calc-notes">"Costing Notes / Specifications"</label>
                <input id="calc-notes" class="input mt-1 text-sm" placeholder="e.g. Requires 2 coats of PU polish, double-pass laser engraving on lid" bind:value=calc.notes />
            </div>
            {move || calc.error.get().map(|e| view! { <div role="alert" class="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm font-medium text-red-700">{e}</div> })}
            {move || calc.success.get().map(|s| view! { <div role="status" class="mt-4 rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm font-medium text-emerald-800">{s}</div> })}
            <div class="mt-6 flex flex-wrap items-center justify-between gap-4 border-t border-(--color-border) pt-4">
                <div class="flex items-center gap-2">
                    <A href="/products" attr:class="btn-secondary">"← Back to Products"</A>
                    {move || (!calc.product_id.get().is_empty()).then(|| view! { <A href=product_href(&calc.code.get()) attr:class="btn-secondary">"View in Product Master"</A> })}
                </div>
                <button type="button" class="btn-primary px-5 py-2.5 text-sm font-bold shadow" disabled=move || calc.busy.get() on:click=move |_| on_save()>
                    {move || if calc.busy.get() { "Saving Costing…".to_string() } else { format!("Save Product ({})", Some(calc.code.get()).filter(|c| !c.is_empty()).unwrap_or_else(|| "Primary".into())) }}
                </button>
            </div>
        </div>
    }
}

/// A modal `<dialog>` that opens as soon as it mounts.
fn show_on_mount(dialog: NodeRef<html::Dialog>) {
    Effect::new(move |_| {
        if let Some(d) = dialog.get() {
            let _ = d.show_modal();
        }
    });
}

/// Saving a product that has other colours: its old and new price, and the
/// other colours that should take the new price (and its costing) too.
#[component]
fn ApplyPriceDialog<N>(calc: Calc, navigate: StoredValue<N, LocalStorage>) -> impl IntoView
where
    N: Fn(&str, leptos_router::NavigateOptions) + 'static,
{
    let dialog = NodeRef::<html::Dialog>::new();
    show_on_mount(dialog);
    let picked = RwSignal::new(Vec::<String>::new());
    let siblings = calc.siblings.get_value();
    let (code, color, old_sp, new_sp) = price_question(calc).unwrap_or_default();
    let all_ids: Vec<String> = siblings.iter().map(|s| s.id.clone()).collect();
    let n = all_ids.len();
    let all = move || picked.with(Vec::len) == n;
    view! {
        <dialog node_ref=dialog on:close=move |_| calc.asking_price.set(false) aria-labelledby="apply-price-title"
            class="m-auto w-[min(92vw,30rem)] rounded-xl bg-white p-5 shadow-2xl backdrop:bg-black/50">
            <h2 id="apply-price-title" class="text-base font-semibold">"Price changed"</h2>
            <p class="mt-2 text-sm">
                <span class="font-mono font-semibold">{code}</span>
                {format!(" ({color}) was ")}
                <span class="font-mono">{format_money(Some(old_sp))}</span>
                ", new price "
                <span class="font-mono font-semibold text-(--color-brand-dark)">{format_money(Some(new_sp))}</span>
                "."
            </p>
            <p class="mt-4 text-xs font-semibold text-(--color-muted)">"Use the new price for these colours too:"</p>
            <ul class="mt-2 divide-y divide-(--color-line) rounded-lg border border-(--color-line)">
                <li>
                    <label class="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-sm font-semibold">
                        <input type="checkbox" prop:checked=all on:change=move |_| picked.set(if all() { Vec::new() } else { all_ids.clone() }) />
                        <span class="flex-1">"All colours"</span>
                        <span class="text-[11px] font-semibold text-(--color-muted)">"Previous SP"</span>
                    </label>
                </li>
                {siblings.into_iter().map(|s| {
                    let (id, id2) = (s.id.clone(), s.id.clone());
                    let on = Memo::new(move |_| picked.with(|p| p.contains(&id)));
                    view! {
                        <li>
                            <label class="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-sm">
                                <input type="checkbox" prop:checked=on on:change=move |_| picked.update(|p| if let Some(i) = p.iter().position(|x| *x == id2) { p.remove(i); } else { p.push(id2.clone()); }) />
                                <span class="flex-1">{s.color.clone()}" "<span class="font-mono text-xs text-(--color-muted)">{s.code.clone()}</span></span>
                                <span class="whitespace-nowrap font-mono text-xs">
                                    <span class=move || if on.get() && s.sp != new_sp { "text-(--color-muted) line-through" } else { "" }>{format_money(Some(s.sp))}</span>
                                    {move || (on.get() && s.sp != new_sp).then(|| view! { <span class="font-semibold text-(--color-brand-dark)">{format!(" → {}", format_money(Some(new_sp)))}</span> })}
                                </span>
                            </label>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <p class="mt-2 text-[11px] text-(--color-muted)">"Ticked colours get this whole costing: cost lines, CP and SP."</p>
            <div class="mt-5 flex justify-end gap-2">
                <button type="button" class="btn-secondary" disabled=move || calc.busy.get() on:click=move |_| if let Some(d) = dialog.get() { d.close() }>"Cancel"</button>
                <button type="button" class="btn-primary" disabled=move || calc.busy.get() on:click=move |_| commit(calc, picked.get_untracked(), nav_box(navigate))>
                    {move || if calc.busy.get() { "Saving…".to_string() } else if picked.with(Vec::is_empty) { "Save this colour only".to_string() } else { format!("Save {} colours", picked.with(Vec::len) + 1) }}
                </button>
            </div>
        </dialog>
    }
}

#[component]
fn LeaveDialog(
    calc: Calc,
    on_save: impl Fn() + 'static,
    on_discard: impl Fn() + 'static,
    on_cancel: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    show_on_mount(dialog);
    view! {
        <dialog node_ref=dialog on:close=move |_| on_cancel() aria-labelledby="leave-title"
            class="m-auto w-[min(92vw,26rem)] rounded-xl bg-white p-5 shadow-2xl backdrop:bg-black/50">
            <h2 id="leave-title" class="text-base font-semibold">"Save changes?"</h2>
            <p class="mt-2 text-sm text-(--color-muted)">"This product has changes that haven't been saved."</p>
            {move || calc.error.get().map(|e| view! { <p role="alert" class="mt-3 rounded-lg border border-red-200 bg-red-50 p-2.5 text-sm text-red-700">{e}</p> })}
            <div class="mt-5 flex justify-end gap-2">
                <button type="button" class="btn-secondary" disabled=move || calc.busy.get() on:click=move |_| if let Some(d) = dialog.get() { d.close() }>"Cancel"</button>
                <button type="button" class="btn-secondary" disabled=move || calc.busy.get() on:click=move |_| on_discard()>"Don't save"</button>
                <button type="button" class="btn-primary" disabled=move || calc.busy.get() on:click=move |_| on_save()>{move || if calc.busy.get() { "Saving…" } else { "Save" }}</button>
            </div>
        </dialog>
    }
}
