//! The hamper builder (port of `hamper-builder.tsx` and `_builder/*`):
//! lines priced on the device as you type, saved through `save_hamper`.

use super::{HamperPage, hamper_href, load_hamper_page};
use crate::components::PageHeader;
use crate::data::{CatalogProduct, use_catalog, use_categories, use_settings};
use crate::db::{self, Q, live_with, me};
use crate::kit::{Combo, FallbackImg, confirm, dstr, js_str, json_num as n, pct_str};
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::images::thumb_url;
use lattice_core::js::{self, dec, f};
use lattice_core::money::{format_money, format_pct};
use lattice_core::numbers::parse_num;
use lattice_core::pricing::{HamperLineInput, HamperOpts, HamperPricing, margin_pct_of, price_hamper, sp_at_margin};
use lattice_core::records::{HamperItem, HamperSummary};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU32, Ordering};

fn optional(v: &str) -> Option<f64> {
    (!v.is_empty()).then(|| js::num_str(v))
}

static NEXT_KEY: AtomicU32 = AtomicU32::new(1);

/// A line as typed: numbers stay strings until saved; margin is a whole percent.
#[derive(Clone, Debug, Default, PartialEq)]
struct Line {
    key: u32,
    product_id: Option<String>,
    product_code: String,
    product_name: String,
    category_name: String,
    source: String,
    qty: String,
    unit_cp: String,
    unit_sp: String,
    target_margin: String,
    /// Margin or price typed by hand: saving makes it the product's price too.
    repriced: bool,
}

impl Line {
    fn blank() -> Self {
        Self { key: NEXT_KEY.fetch_add(1, Ordering::Relaxed), qty: "1".into(), ..Default::default() }
    }

    fn from_item(i: &HamperItem) -> Self {
        Self {
            key: NEXT_KEY.fetch_add(1, Ordering::Relaxed),
            product_id: i.product_id.clone(),
            product_code: i.product_code.clone().unwrap_or_default(),
            product_name: i.product_name.clone(),
            category_name: i.category_name.clone().unwrap_or_default(),
            source: i.source.clone().unwrap_or_default(),
            qty: dstr(i.qty),
            unit_cp: dstr(i.unit_cp),
            unit_sp: dstr(i.unit_sp),
            target_margin: pct_str(i.target_margin),
            repriced: false,
        }
    }

    /// `populateProductRow()`: a product fills its line; none clears it.
    fn pick(&mut self, p: Option<&CatalogProduct>) {
        self.repriced = false;
        let Some(p) = p else {
            self.product_id = None;
            self.product_code.clear();
            self.product_name.clear();
            return;
        };
        self.product_id = Some(p.id.clone());
        self.product_code = p.code.clone();
        self.product_name = p.name.clone();
        self.category_name = p.category_name.clone().unwrap_or_default();
        self.source = p.source.clone().unwrap_or_default();
        self.unit_cp = dstr(p.cost_price);
        self.unit_sp = dstr(p.default_sp);
        self.target_margin = pct_str(p.target_margin);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Price {
    Cp,
    Sp,
    Margin,
}

/// Margin and unit price follow each other: a margin prices the line from its
/// cost, a typed price works out the margin. A new cost keeps the price and
/// moves the margin, as a cost change does in the calculator.
fn price_edit(l: &mut Line, field: Price, value: String) {
    let cp = parse_num(if field == Price::Cp { &value } else { &l.unit_cp });
    if field == Price::Margin {
        let sp = if value.is_empty() { None } else { sp_at_margin(cp, parse_num(&value)) };
        l.target_margin = value;
        l.repriced = true;
        if let Some(sp) = sp {
            l.unit_sp = dstr(sp);
        }
        return;
    }
    let margin = margin_pct_of(cp, parse_num(if field == Price::Sp { &value } else { &l.unit_sp }));
    if field == Price::Sp {
        l.unit_sp = value;
        l.repriced = true;
    } else {
        l.unit_cp = value;
    }
    if let Some(m) = margin {
        l.target_margin = dstr(m);
    }
}

/// What a product looks like in the search box: name plus code, so typing
/// either narrows the list, and the text maps back to exactly one product.
fn product_label(name: &str, code: &str) -> String {
    if code.is_empty() { name.to_string() } else { format!("{name} — {code}") }
}

/// Hamper details as typed; discount is a whole percent.
#[derive(Clone, Copy)]
struct Fields {
    name: RwSignal<String>,
    collection: RwSignal<String>,
    status: RwSignal<String>,
    notes: RwSignal<String>,
    target_sp: RwSignal<String>,
    discount: RwSignal<String>,
    final_sp: RwSignal<String>,
}

impl Fields {
    fn new(h: Option<&HamperSummary>) -> Self {
        let opt = |v: Option<Decimal>| v.map(dstr).unwrap_or_default();
        Self {
            name: RwSignal::new(h.map(|h| h.name.clone()).unwrap_or_default()),
            collection: RwSignal::new(h.and_then(|h| h.collection.clone()).unwrap_or_default()),
            status: RwSignal::new(h.map_or_else(|| "Draft".into(), |h| h.status.clone())),
            notes: RwSignal::new(h.and_then(|h| h.notes.clone()).unwrap_or_default()),
            target_sp: RwSignal::new(opt(h.and_then(|h| h.target_sp))),
            discount: RwSignal::new(h.map_or_else(|| "0".into(), |h| pct_str(h.discount_pct))),
            final_sp: RwSignal::new(opt(h.and_then(|h| h.final_catalogue_sp))),
        }
    }

    fn opts(&self) -> HamperOpts {
        HamperOpts {
            discount_pct: dec(js::num_str(&self.discount.get()) / 100.0),
            target_sp: optional(&self.target_sp.get()).map(dec),
            final_catalogue_sp: optional(&self.final_sp.get()).map(dec),
        }
    }
}

/// `/hampers/new`.
#[component]
pub fn NewHamper() -> impl IntoView {
    view! {
        <PageHeader title="New hamper" subtitle="The code is assigned when you save, continuing from the last one">""</PageHeader>
        <Builder hamper=None />
    }
}

/// `/hampers/H128/edit`: the builder for a saved hamper.
#[component]
pub fn EditHamper() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let code = Memo::new(move |_| {
        params.with(|p| p.get("code").map(|c| js_sys::decode_uri_component(&c).map(String::from).unwrap_or(c)).unwrap_or_default())
    });
    view! {
        {move || {
            let code = code.get();
            let c = code.clone();
            let page = live_with(format!("hamper:{code}"), move || load_hamper_page(c.clone()));
            // The builder starts from the first copy (cached or fresh) and then
            // owns its state; a background refresh must not wipe what is typed.
            let first = RwSignal::new(None::<HamperPage>);
            Effect::new(move |_| {
                if first.with_untracked(Option::is_none)
                    && let Some(p) = page.data.get()
                {
                    first.set(Some(p));
                }
            });
            let duplicated = move || query.with_untracked(|q| q.get("duplicated")).filter(|d| !d.is_empty());
            view! {
                <Show
                    when=move || first.with(Option::is_some)
                    fallback=move || match page.error.get() {
                        Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                        None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
                    }
                >
                    {move || first.get_untracked().map(|p| {
                        let h = p.hamper.clone();
                        view! {
                            <PageHeader title=h.name.clone() subtitle=h.code.clone()>
                                <span class="badge">{h.status.clone()}</span>
                            </PageHeader>
                            {duplicated().map(|from| view! {
                                <p class="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">
                                    {format!("Copied from {from}. Adjust it and save.")}
                                </p>
                            })}
                            <Builder hamper=Some(p) />
                        }
                    })}
                </Show>
            }
        }}
    }
}

#[component]
fn Builder(hamper: Option<HamperPage>) -> impl IntoView {
    let can_edit = me().role.can_manage();
    let catalog = use_catalog();
    let categories = use_categories();
    let settings = use_settings();
    let navigate = StoredValue::new_local(use_navigate());
    let go = move |to: String| navigate.with_value(|nav| nav(&to, Default::default()));

    let saved = StoredValue::new(hamper.as_ref().map(|p| p.hamper.clone()));
    let fields = Fields::new(hamper.as_ref().map(|p| &p.hamper));
    let first_lines: Vec<Line> = hamper.as_ref().map(|p| p.items.iter().map(|i| Line::from_item(&i.item)).collect()).unwrap_or_default();
    let lines = RwSignal::new(if first_lines.is_empty() { vec![Line::blank()] } else { first_lines });
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(None::<&'static str>);

    // categoryCountsAsItem(): anything not listed in Settings counts.
    let counts = Memo::new(move |_| {
        categories.data.with(|c| {
            c.iter().flatten().map(|c| (c.name.trim().to_lowercase(), c.counts_as_item)).collect::<HashMap<_, _>>()
        })
    });
    let totals = Memo::new(move |_| {
        let counts = counts.get();
        let inputs: Vec<HamperLineInput> = lines.with(|ls| {
            ls.iter()
                .map(|l| HamperLineInput {
                    qty: parse_num(&l.qty),
                    unit_cp: parse_num(&l.unit_cp),
                    unit_sp: parse_num(&l.unit_sp),
                    counts_as_item: counts.get(&l.category_name.trim().to_lowercase()).copied().unwrap_or(true),
                })
                .collect()
        });
        price_hamper(&inputs, &fields.opts())
    });
    let products = Memo::new(move |_| catalog.data.get().unwrap_or_default());
    let all_labels = Memo::new(move |_| products.with(|ps| ps.iter().map(|p| product_label(&p.name, &p.code)).collect::<Vec<_>>()));
    let category_options = Memo::new(move |_| {
        std::iter::once("All categories".to_string())
            .chain(categories.data.with(|c| c.iter().flatten().map(|c| c.name.clone()).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    });
    let filled = move || lines.with(|ls| ls.iter().filter(|l| !l.product_name.is_empty()).count());

    let save = move || {
        error.set(None);
        let (name, status) = (fields.name.get_untracked().trim().to_string(), fields.status.get_untracked().trim().to_string());
        let ls = lines.get_untracked();
        let kept: Vec<&Line> = ls.iter().filter(|l| !l.product_name.trim().is_empty()).collect();
        let final_sp = optional(&fields.final_sp.get_untracked());
        let status = if status.is_empty() { "Draft".to_string() } else { status };
        let problem = if name.is_empty() {
            Some("Hamper name is required".to_string())
        } else if kept.is_empty() {
            Some("Add at least one product to the hamper".to_string())
        } else if status != "Draft" && final_sp.is_none() {
            // Only once it leaves Draft: a hamper still being costed has no price yet.
            Some(format!("A Final Catalogue SP is required before a hamper can be marked {status}."))
        } else {
            None
        };
        if let Some(p) = problem {
            return error.set(Some(p));
        }
        let text = |s: String| Some(s.trim().to_string()).filter(|s| !s.is_empty());
        let payload = json!({
            "id": saved.with_value(|h| h.as_ref().map(|h| h.id.clone())),
            "name": name,
            "collection": text(fields.collection.get_untracked()),
            "status": status,
            "target_sp": optional(&fields.target_sp.get_untracked()).map(n),
            "notes": text(fields.notes.get_untracked()),
            "discount_pct": n(js::num_str(&fields.discount.get_untracked()) / 100.0),
            "final_catalogue_sp": final_sp.map(n),
            "lines": kept.iter().map(|l| json!({
                "product_id": l.product_id,
                "product_code": text(l.product_code.clone()),
                "product_name": l.product_name.trim(),
                "category_name": text(l.category_name.clone()),
                "source": text(l.source.clone()),
                "qty": n(js::num_str(&l.qty)),
                "unit_cp": n(js::num_str(&l.unit_cp)),
                "unit_sp": n(js::num_str(&l.unit_sp)),
                "target_margin": n(js::num_str(&l.target_margin) / 100.0),
                "reprice": l.repriced,
            })).collect::<Vec<_>>(),
        });
        // Prices typed on a line become the product's price too (admins only:
        // Product Master is admin-only under RLS; a manager's stay on the hamper).
        let repriced: Vec<Repriced> = kept
            .iter()
            .filter(|l| l.repriced)
            .filter_map(|l| {
                l.product_id.clone().map(|id| Repriced {
                    product_id: id,
                    unit_sp: js::num_str(&l.unit_sp),
                    target_margin: js::num_str(&l.target_margin) / 100.0,
                })
            })
            .collect();
        let admin = me().role.is_admin();
        let old_code = saved.with_value(|h| h.as_ref().map(|h| h.code.clone()));
        busy.set(Some("save"));
        spawn_local(async move {
            if admin && !repriced.is_empty() {
                if let Err(e) = push_line_prices(&repriced).await {
                    busy.set(None);
                    return error.set(Some(format!("Could not update the product prices: {e}")));
                }
                db::cache_drop(&["products", "catalog"]).await;
            }
            match db::rpc::<_, String>("save_hamper", &json!({ "p": payload })).await {
                Ok(code) => {
                    let mut stale = vec!["hampers".to_string(), "dashboard".into(), format!("hamper:{code}")];
                    stale.extend(old_code.map(|c| format!("hamper:{c}")));
                    db::cache_drop(&stale.iter().map(String::as_str).collect::<Vec<_>>()).await;
                    // Saving is the end of the job: back to the list.
                    go("/hampers".into());
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };
    let duplicate = move || {
        let Some(code) = saved.with_value(|h| h.as_ref().map(|h| h.code.clone())) else { return };
        error.set(None);
        busy.set(Some("dup"));
        spawn_local(async move {
            match db::rpc::<_, String>("duplicate_hamper", &json!({ "p_code": code })).await {
                Ok(new) => {
                    db::cache_drop(&["hampers", "dashboard"]).await;
                    let enc = |s: &str| String::from(js_sys::encode_uri_component(s));
                    go(format!("{}/edit?duplicated={}", hamper_href(&new), enc(&code)));
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };
    let bin = move || {
        let Some((id, code, name)) = saved.with_value(|h| h.as_ref().map(|h| (h.id.clone(), h.code.clone(), h.name.clone()))) else {
            return;
        };
        if !confirm(&format!("Move {name} to the Bin? It will stay in the Bin for 30 days and can be restored anytime.")) {
            return;
        }
        error.set(None);
        busy.set(Some("bin"));
        spawn_local(async move {
            let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
            match Q::from("hampers").eq("id", &id).update(&json!({ "deleted_at": now }), None).await {
                Ok(_) => {
                    db::cache_drop(&["hampers", "dashboard", &format!("hamper:{code}")]).await;
                    go("/hampers?deleted=1".into());
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    let statuses = move || {
        let mut list = settings.data.get().map(|s| s.hamper_statuses).unwrap_or_default();
        let current = fields.status.get();
        if !list.contains(&current) {
            list.push(current);
        }
        list
    };
    let collections = move || {
        let mut list = settings.data.get().map(|s| s.collections).unwrap_or_default();
        let current = fields.collection.get();
        if !current.is_empty() && !list.contains(&current) {
            list.push(current);
        }
        list
    };
    let cancel = saved.with_value(|h| h.as_ref().map_or_else(|| "/hampers".to_string(), |h| hamper_href(&h.code)));
    let existing = saved.get_value();
    let disabled = !can_edit;

    view! {
        <div class="space-y-4">
            <section class="card p-4">
                <div class="grid gap-4 md:grid-cols-3">
                    <div class="md:col-span-2">
                        <label class="label" for="name">"Hamper name"</label>
                        <input id="name" class="input mt-1" required disabled=disabled prop:value=fields.name on:input=move |e| fields.name.set(event_target_value(&e)) />
                    </div>
                    <div>
                        <label class="label" for="status">"Status"</label>
                        <select id="status" class="select mt-1" disabled=disabled on:change=move |e| fields.status.set(event_target_value(&e))>
                            {move || statuses().into_iter().map(|s| {
                                let on = s == fields.status.get_untracked();
                                let label = s.clone();
                                view! { <option value=s selected=on>{label}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <div>
                        <label class="label" for="collection">"Collection / occasion"</label>
                        <input id="collection" list="collections" class="input mt-1" disabled=disabled prop:value=fields.collection on:input=move |e| fields.collection.set(event_target_value(&e)) />
                        <datalist id="collections">
                            {move || collections().into_iter().map(|c| view! { <option value=c></option> }).collect_view()}
                        </datalist>
                    </div>
                    <div class="md:col-span-2">
                        <label class="label" for="notes">"Notes"</label>
                        <input id="notes" class="input mt-1" disabled=disabled prop:value=fields.notes on:input=move |e| fields.notes.set(event_target_value(&e)) />
                    </div>
                </div>
            </section>

            <PricingStrip fields=fields totals=totals disabled=disabled />

            <section class="card overflow-hidden">
                <div class="flex items-center justify-between border-b border-(--color-line) px-4 py-2.5">
                    <h2 class="text-sm font-semibold">
                        "Products"
                        <span class="ml-2 font-normal text-(--color-muted)">{move || { let n = filled(); format!("{n} line{}", if n == 1 { "" } else { "s" }) }}</span>
                    </h2>
                    {can_edit.then(|| view! {
                        <button type="button" class="btn-secondary" on:click=move |_| lines.update(|ls| ls.push(Line::blank()))>"Add line"</button>
                    })}
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full min-w-[1060px] border-collapse text-sm">
                        <thead>
                            <tr>
                                <th class="th w-[170px]">"Category"</th>
                                <th class="th w-[280px]">"Product"</th>
                                <th class="th w-[110px]">"Code"</th>
                                <th class="th w-[80px] num">"Qty"</th>
                                <th class="th w-[110px] num">"Unit cost"</th>
                                <th class="th w-[110px] num">"Total cost"</th>
                                <th class="th w-[90px] num">"Margin %"</th>
                                <th class="th w-[110px] num">"Unit price"</th>
                                <th class="th w-[110px] num">"Total price"</th>
                                {can_edit.then(|| view! { <th class="th w-[90px]"></th> })}
                            </tr>
                        </thead>
                        <tbody>
                            <For each=move || lines.with(|ls| ls.iter().map(|l| l.key).collect::<Vec<_>>()) key=|k| *k let:key>
                                <LineRow key=key lines=lines products=products all_labels=all_labels category_options=category_options can_edit=can_edit />
                            </For>
                        </tbody>
                    </table>
                </div>
            </section>

            {move || error.get().map(|e| view! { <p role="alert" class="text-sm text-red-700">{e}</p> })}

            {can_edit.then(|| view! {
                <div class="flex flex-wrap items-center gap-2">
                    <button type="button" class="btn-primary" disabled=move || busy.get().is_some() on:click=move |_| save()>
                        {move || match busy.get() {
                            Some("save") => "Saving…",
                            _ if saved.with_value(Option::is_some) => "Save changes",
                            _ => "Create hamper",
                        }}
                    </button>
                    <A href=cancel attr:class="btn-secondary">"Cancel"</A>
                </div>
            })}
        </div>

        {existing.filter(|_| can_edit).map(|h| {
            let image = format!("{}/image", hamper_href(&h.code));
            let quote = format!("/quotes/new?hamper={}", String::from(js_sys::encode_uri_component(&h.code)));
            let has_image = h.image_url.is_some();
            view! {
                <div class="mt-4 flex flex-wrap items-center gap-2">
                    {h.image_url.clone().map(|url| view! {
                        <img src=url alt=format!("{} image", h.name) class="h-16 w-16 rounded-md border border-(--color-line) object-cover" />
                    })}
                    <A href=image attr:class="btn-secondary">{if has_image { "Edit image" } else { "Design image" }}</A>
                    <A href=quote attr:class="btn-secondary">"Add to a quote"</A>
                    <button type="button" class="btn-secondary" disabled=move || busy.get().is_some() on:click=move |_| duplicate()>
                        {move || if busy.get() == Some("dup") { "Duplicating…" } else { "Duplicate" }}
                    </button>
                    <button type="button" class="btn-danger ml-auto" disabled=move || busy.get().is_some() on:click=move |_| bin()>
                        {move || if busy.get() == Some("bin") { "Moving to Bin…" } else { "Move to Bin" }}
                    </button>
                </div>
            }
        })}
    }
}

fn tone(v: Option<Decimal>) -> &'static str {
    match v {
        None => "",
        Some(v) if v < Decimal::ZERO => "text-red-700",
        Some(_) => "text-green-800",
    }
}

#[component]
fn Cell(label: &'static str, #[prop(into)] value: Signal<String>, #[prop(optional, into)] tone: Signal<&'static str>, #[prop(optional)] strong: bool) -> impl IntoView {
    let size = if strong { "text-base font-semibold" } else { "text-sm font-medium" };
    view! {
        <div>
            <div class="label">{label}</div>
            <div class=move || format!("mt-1 tabular-nums {size} {}", tone.get())>{value}</div>
        </div>
    }
}

#[component]
fn NumField(id: &'static str, label: &'static str, value: RwSignal<String>, disabled: bool, #[prop(optional)] hint: &'static str) -> impl IntoView {
    view! {
        <div class="w-30">
            <label class="label" for=id title=hint>{label}</label>
            <input id=id inputmode="decimal" class="input input-num mt-1" disabled=disabled prop:value=value on:input=move |e| value.set(event_target_value(&e)) />
        </div>
    }
}

/// One strip between the details and the products, so the numbers that move
/// while you edit lines stay on screen.
#[component]
fn PricingStrip(fields: Fields, totals: Memo<HamperPricing>, disabled: bool) -> impl IntoView {
    let t = move |pick: fn(&HamperPricing) -> Option<Decimal>| Signal::derive(move || totals.with(pick));
    let money = move |pick: fn(&HamperPricing) -> Option<Decimal>| Signal::derive(move || totals.with(pick).map_or_else(|| "—".into(), |v| format_money(Some(v))));
    view! {
        <section class="card p-4">
            <div class="flex flex-wrap items-end gap-x-6 gap-y-3">
                <Cell label="Total cost" value=money(|t| Some(t.total_cp)) />
                <Cell label="Base SP" value=money(|t| Some(t.base_sp)) />
                <NumField id="discount" label="Discount %" value=fields.discount disabled=disabled />
                <NumField id="targetSp" label="Target SP" value=fields.target_sp disabled=disabled />
                <Cell label="SP after discount" value=money(|t| Some(t.sp_after_discount)) />
                <Cell label="Variance" value=money(|t| t.variance) tone=Signal::derive(move || tone(t(|t| t.variance).get())) />
                <NumField id="finalSp" label="Final catalogue SP" value=fields.final_sp disabled=disabled hint="Never overwritten by a price refresh" />
                <Cell label="Items" value=Signal::derive(move || js_str(js::round2(f(totals.with(|t| t.number_of_items))))) />
                <Cell label="Gross profit" value=money(|t| t.gross_profit) tone=Signal::derive(move || tone(t(|t| t.gross_profit).get())) />
                <Cell
                    label="Final margin"
                    value=Signal::derive(move || totals.with(|t| t.final_margin).map_or_else(|| "—".into(), |m| format_pct(Some(m), 1)))
                    tone=Signal::derive(move || tone(t(|t| t.final_margin).get()))
                    strong=true
                />
            </div>
        </section>
    }
}

#[component]
fn LineRow(
    key: u32,
    lines: RwSignal<Vec<Line>>,
    products: Memo<Vec<CatalogProduct>>,
    all_labels: Memo<Vec<String>>,
    category_options: Memo<Vec<String>>,
    can_edit: bool,
) -> impl IntoView {
    let line = Memo::new(move |_| lines.with(|ls| ls.iter().find(|l| l.key == key).cloned().unwrap_or_default()));
    let update = move |f: &dyn Fn(&mut Line)| {
        lines.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.key == key) {
                f(l);
            }
        });
    };
    let position = Memo::new(move |_| lines.with(|ls| (ls.iter().position(|l| l.key == key).unwrap_or(0), ls.len())));
    // Filtered here on the device: the spreadsheet rebuilt the dropdown
    // server-side on every category change, which is what made it slow.
    let options = Memo::new(move |_| {
        let wanted = line.with(|l| l.category_name.trim().to_lowercase());
        products.with(|ps| {
            ps.iter()
                .filter(|p| wanted.is_empty() || p.category_name.as_deref().unwrap_or("").trim().to_lowercase() == wanted)
                .map(|p| product_label(&p.name, &p.code))
                .collect::<Vec<_>>()
        })
    });
    let selected = Memo::new(move |_| {
        let id = line.with(|l| l.product_id.clone());
        id.and_then(|id| products.with(|ps| ps.iter().find(|p| p.id == id).cloned()))
    });
    let by_label = move |text: &str| {
        products.with_untracked(|ps| {
            let label = |p: &&CatalogProduct| product_label(&p.name, &p.code) == text;
            let wanted = line.with_untracked(|l| l.category_name.trim().to_lowercase());
            ps.iter()
                .filter(|p| wanted.is_empty() || p.category_name.as_deref().unwrap_or("").trim().to_lowercase() == wanted)
                .find(label)
                .or_else(|| ps.iter().find(label))
                .cloned()
        })
    };
    let total = move |a: fn(&Line) -> &String| {
        move || {
            let x = line.with(|l| js::num_str(&l.qty) * js::num_str(a(l)));
            if x != 0.0 && !x.is_nan() { format_money(Some(dec(x))) } else { "—".into() }
        }
    };
    let what = move || line.with(|l| if l.product_name.is_empty() { "line".to_string() } else { l.product_name.clone() });
    let cell = move |field: Option<Price>, label: &'static str, get: fn(&Line) -> String| {
        view! {
            <td class="td">
                <input aria-label=label inputmode="decimal" class="input input-num" disabled=!can_edit prop:value=move || line.with(get)
                    on:input=move |e| {
                        let v = event_target_value(&e);
                        match field {
                            None => update(&|l| l.qty = v.clone()),
                            Some(p) => update(&|l| price_edit(l, p, v.clone())),
                        }
                    } />
            </td>
        }
    };

    view! {
        <tr>
            <td class="td">
                <Combo
                    label="Category"
                    placeholder="All categories"
                    value=Signal::derive(move || line.with(|l| l.category_name.clone()))
                    options=category_options
                    disabled=!can_edit
                    on_pick=move |category: String| {
                        let cat = if category == "All categories" { String::new() } else { category };
                        let still = cat.is_empty() || selected.with_untracked(|p| p.as_ref().and_then(|p| p.category_name.clone())) == Some(cat.clone());
                        // Clearing a mismatched product mirrors the sheet's onEdit handler.
                        update(&|l| {
                            l.category_name = cat.clone();
                            if !still {
                                l.product_id = None;
                                l.product_code.clear();
                                l.product_name.clear();
                            }
                        });
                    }
                />
            </td>
            <td class="td">
                <div class="flex items-center gap-2">
                    {move || selected.get().and_then(|p| p.image_url).map(|url| view! {
                        <div class="relative h-8 w-8 shrink-0 overflow-hidden rounded-md border border-slate-200 bg-slate-100 shadow-2xs">
                            <FallbackImg src=thumb_url(&url) fallback=url alt="" class="h-full w-full object-cover" />
                        </div>
                    })}
                    <div class="min-w-0 flex-1">
                        <Combo
                            label="Product"
                            placeholder="Search products…"
                            value=Signal::derive(move || line.with(|l| if l.product_name.is_empty() { String::new() } else { product_label(&l.product_name, &l.product_code) }))
                            options=options
                            all=all_labels
                            disabled=!can_edit
                            on_pick=move |text: String| {
                                if let Some(p) = by_label(&text) {
                                    update(&|l| l.pick(Some(&p)));
                                } else if text.is_empty() {
                                    update(&|l| l.pick(None));
                                }
                            }
                        />
                        // A product since retired still shows on saved lines.
                        <Show when=move || line.with(|l| l.product_id.is_none() && !l.product_name.is_empty())>
                            <div class="mt-0.5 text-xs text-(--color-muted)">"No longer in Product Master"</div>
                        </Show>
                    </div>
                </div>
            </td>
            <td class="td font-mono text-xs text-(--color-muted)">{move || line.with(|l| if l.product_code.is_empty() { "—".to_string() } else { l.product_code.clone() })}</td>
            {cell(None, "Quantity", |l| l.qty.clone())}
            {cell(Some(Price::Cp), "Unit cost", |l| l.unit_cp.clone())}
            <td class="td num text-(--color-muted)">{total(|l| &l.unit_cp)}</td>
            {cell(Some(Price::Margin), "Target margin", |l| l.target_margin.clone())}
            {cell(Some(Price::Sp), "Unit price", |l| l.unit_sp.clone())}
            <td class="td num font-medium">{total(|l| &l.unit_sp)}</td>
            {can_edit.then(|| {
                let shift = move |delta: isize| {
                    lines.update(|ls| {
                        if let Some(i) = ls.iter().position(|l| l.key == key) {
                            let j = i.checked_add_signed(delta).filter(|j| *j < ls.len());
                            if let Some(j) = j {
                                ls.swap(i, j);
                            }
                        }
                    });
                };
                let btn = "rounded px-1.5 py-0.5 text-(--color-muted) disabled:opacity-30";
                view! {
                    <td class="td">
                        <div class="flex items-center justify-end gap-0.5">
                            <button type="button" class=format!("{btn} hover:bg-paper") title=move || format!("Move {} up", what()) aria-label=move || format!("Move {} up", what())
                                disabled=move || position.get().0 == 0 on:click=move |_| shift(-1)>"↑"</button>
                            <button type="button" class=format!("{btn} hover:bg-paper") title=move || format!("Move {} down", what()) aria-label=move || format!("Move {} down", what())
                                disabled=move || { let (i, n) = position.get(); i + 1 >= n } on:click=move |_| shift(1)>"↓"</button>
                            <button type="button" class=format!("{btn} hover:bg-red-50 hover:text-red-700") title=move || format!("Remove {}", what()) aria-label=move || format!("Remove {}", what())
                                on:click=move |_| lines.update(|ls| if ls.len() == 1 { *ls = vec![Line::blank()] } else { ls.retain(|l| l.key != key) })>"×"</button>
                        </div>
                    </td>
                }
            })}
        </tr>
    }
}

/// A hamper line whose margin or price was typed by hand.
struct Repriced {
    product_id: String,
    unit_sp: f64,
    target_margin: f64,
}

#[derive(serde::Deserialize)]
struct ProductPrice {
    id: String,
    code: String,
    name: String,
    #[serde(deserialize_with = "lattice_core::lenient::decimal")]
    cost_price: Decimal,
}

#[derive(serde::Deserialize)]
struct SheetName {
    id: String,
    product_id: String,
    product_name: String,
}

/// Makes a price set on a hamper line the product's own price in Product
/// Master and on its cost sheet (port of `push-prices.ts`). The line's margin
/// is the calculator's markup. Other hampers keep theirs until Refresh prices.
async fn push_line_prices(lines: &[Repriced]) -> Result<(), ApiError> {
    // The same product twice: the later line wins.
    let by_product: BTreeMap<&str, &Repriced> = lines.iter().map(|l| (l.product_id.as_str(), l)).collect();
    if by_product.is_empty() {
        return Ok(());
    }
    let ids: Vec<&str> = by_product.keys().copied().collect();
    let (pq, sq) = (
        Q::from("products").select("id,code,name,cost_price").in_list("id", &ids),
        Q::from("product_cost_sheets").select("id,product_id,product_name").in_list("product_id", &ids),
    );
    let (products, sheets) = futures::join!(pq.rows::<ProductPrice>(), sq.rows::<SheetName>());
    let (products, sheets) = (products?, sheets?);
    let markup = |l: &Repriced| n(js::round2(l.target_margin * 100.0));
    let product_rows: Vec<Value> = products
        .iter()
        .filter_map(|p| {
            let l = by_product.get(p.id.as_str())?;
            let (sp, cp) = (l.unit_sp, f(p.cost_price));
            Some(json!({
                "id": p.id, "code": p.code, "name": p.name, "markup_pct": markup(l), "default_sp": n(sp),
                "target_margin": n(if sp > 0.0 { (sp - cp) / sp } else { 0.0 }),
            }))
        })
        .collect();
    let sheet_rows: Vec<Value> = sheets
        .iter()
        .filter_map(|s| {
            let l = by_product.get(s.product_id.as_str())?;
            Some(json!({ "id": s.id, "product_name": s.product_name, "markup_pct": markup(l), "calculated_sp": n(l.unit_sp) }))
        })
        .collect();
    if !product_rows.is_empty() {
        db::insert("products", &product_rows, Some("id"), None).await?;
    }
    if !sheet_rows.is_empty() {
        db::insert("product_cost_sheets", &sheet_rows, Some("id"), None).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margin_and_price_follow_each_other() {
        let mut l = Line { unit_cp: "100".into(), unit_sp: "150".into(), ..Line::blank() };
        price_edit(&mut l, Price::Margin, "40".into());
        assert_eq!((l.unit_sp.as_str(), l.repriced), ("170", true)); // 100 / 0.6 = 166.67 → next 10
        let mut l = Line { unit_cp: "100".into(), ..Line::blank() };
        price_edit(&mut l, Price::Sp, "150".into());
        assert_eq!((l.target_margin.as_str(), l.repriced), ("33.33", true));
        price_edit(&mut l, Price::Cp, "120".into());
        assert_eq!((l.target_margin.as_str(), l.unit_sp.as_str()), ("20", "150"));
        price_edit(&mut l, Price::Margin, String::new());
        assert_eq!((l.target_margin.as_str(), l.unit_sp.as_str()), ("", "150"));
    }
}
