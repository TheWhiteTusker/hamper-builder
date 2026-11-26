//! Who created, edited, binned, restored or purged each product, and what
//! changed (port of `products/_history/*`; rows from 0026_product_history.sql,
//! which only admins can read).

mod page;

pub use page::ProductHistoryPage;

use crate::data::use_categories;
use crate::db::{Q, live_with};
use crate::kit::{decode_code, product_href, when};
use lattice_core::js::dec;
use lattice_core::money::format_money;
use lattice_core::records::Category;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const HISTORY_SELECT: &str = "*,profiles(full_name)";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Who {
    pub full_name: Option<String>,
}

/// A row of product_history, with who made it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryEntry {
    pub id: i64,
    pub product_id: String,
    pub product_code: String,
    pub product_name: String,
    pub action: String,
    /// field → [old, new]
    pub changes: Map<String, Value>,
    pub changed_at: String,
    pub backfilled: bool,
    pub profiles: Option<Who>,
}

fn verb(action: &str) -> &'static str {
    match action {
        "created" => "created",
        "deleted" => "moved to Bin",
        "restored" => "restored from Bin",
        "purged" => "deleted permanently",
        _ => "edited",
    }
}

fn label(field: &str) -> &str {
    match field {
        "code" => "Code",
        "name" => "Name",
        "category_id" => "Category",
        "source" => "Source",
        "cost_price" => "Cost price",
        "markup_pct" => "Markup",
        "default_sp" => "Selling price",
        "is_active" => "Active",
        "colors" => "Colour",
        "image_url" => "Main photo",
        "material_total" => "Material cost",
        "hardware_total" => "Hardware cost",
        "finishing_total" => "Finishing cost",
        "machine_total" => "Machine cost",
        "stage_overheads" => "Overheads",
        "notes" => "Notes",
        f => f,
    }
}

const MONEY: [&str; 6] =
    ["cost_price", "default_sp", "material_total", "hardware_total", "finishing_total", "machine_total"];
/// Too long or opaque to show as from → to.
const CHANGED_ONLY: [&str; 2] = ["image_url", "stage_overheads"];

fn show(field: &str, v: &Value, categories: &[Category]) -> String {
    let number = || v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0);
    match v {
        Value::Null => "—".into(),
        Value::String(s) if s.is_empty() => "—".into(),
        _ if MONEY.contains(&field) => format_money(Some(dec(number()))),
        _ if field == "markup_pct" => format!("{}%", v.as_str().map_or_else(|| v.to_string(), str::to_string)),
        Value::Bool(b) => (if *b { "Yes" } else { "No" }).into(),
        Value::String(id) if field == "category_id" => {
            categories.iter().find(|c| &c.id == id).map_or_else(|| "a deleted category".into(), |c| c.name.clone())
        }
        Value::Array(xs) => {
            let s = xs
                .iter()
                .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                .collect::<Vec<_>>()
                .join(", ");
            if s.is_empty() { "—".into() } else { s }
        }
        _ => {
            let s = v.as_str().map_or_else(|| v.to_string(), str::to_string);
            if s.chars().count() > 60 { format!("{}…", s.chars().take(60).collect::<String>()) } else { s }
        }
    }
}

/// Newest first. `show_product` names the product on each line (the all-products page).
#[component]
pub fn HistoryList(
    entries: Vec<HistoryEntry>,
    categories: Vec<Category>,
    #[prop(optional)] show_product: bool,
) -> impl IntoView {
    let line = move |e: HistoryEntry| {
        let who = e.profiles.as_ref().and_then(|p| p.full_name.clone());
        let did = if e.backfilled && e.action == "edited" { "last edited" } else { verb(&e.action) };
        let older = if who.is_some() {
            "From before history tracking: the person who first saved its costing."
        } else {
            "From before history tracking: only the date is known."
        };
        let who = who.unwrap_or_else(|| (if e.backfilled { "Someone" } else { "Unknown" }).into());
        let product = show_product.then(|| {
            let code = if e.action == "purged" {
                view! { <span class="font-mono">{e.product_code.clone()}</span> }.into_any()
            } else {
                view! { <A href=product_href(&e.product_code) attr:class="font-mono font-medium hover:underline">{e.product_code.clone()}</A> }.into_any()
            };
            view! { " " {code} " " <span class="text-(--color-muted)">{e.product_name.clone()}</span> }
        });
        let changes = e
            .changes
            .iter()
            .map(|(field, pair)| {
                let (from, to) = (pair.get(0).unwrap_or(&Value::Null), pair.get(1).unwrap_or(&Value::Null));
                let what = if CHANGED_ONLY.contains(&field.as_str()) {
                    " changed".to_string()
                } else {
                    format!(": {} → {}", show(field, from, &categories), show(field, to, &categories))
                };
                view! { <li><span class="font-medium text-(--color-ink)">{label(field).to_string()}</span>{what}</li> }
            })
            .collect_view();
        view! {
            <li class="flex flex-wrap items-start gap-x-4 gap-y-1 px-4 py-3">
                <div class="min-w-0 flex-1">
                    <p class="text-sm">
                        <strong class="font-semibold">{who}</strong> " " {did} {product}
                        {e.backfilled.then(|| view! { <span class="badge ml-2" title=older>"older record"</span> })}
                    </p>
                    {(!e.changes.is_empty()).then(|| view! { <ul class="mt-1 space-y-0.5 text-xs text-(--color-muted)">{changes}</ul> })}
                </div>
                <time datetime=e.changed_at.clone() class="whitespace-nowrap text-xs text-(--color-muted)">{when(&e.changed_at)}</time>
            </li>
        }
    };
    view! { <ul class="divide-y divide-line">{entries.into_iter().map(line).collect_view()}</ul> }
}

/// "History" card for one product: its latest `shown` changes, and a link to the rest.
#[component]
pub fn ProductHistory(product_id: String, #[prop(default = 10)] shown: usize) -> impl IntoView {
    let categories = use_categories();
    let id = product_id.clone();
    let history = live_with(format!("history:{product_id}:{shown}"), move || {
        let id = id.clone();
        async move {
            let q = Q::from("product_history").select(HISTORY_SELECT).eq("product_id", &id).order_desc("changed_at");
            let (rows, count) = futures::future::join(q.clone().limit(shown).rows::<HistoryEntry>(), q.count()).await;
            Ok((rows?, count?))
        }
    });
    view! {
        <section class="card">
            <div class="flex items-center justify-between border-b border-line px-4 py-3">
                <h2 class="font-semibold">"History"</h2>
                {move || history.data.get().filter(|(_, n)| usize::try_from(*n).unwrap_or(0) > shown).map(|(_, n)| view! {
                    <A href=format!("/products/history?product={product_id}") attr:class="text-sm font-medium text-(--color-brand) hover:underline">
                        {format!("See all {n} changes")}
                    </A>
                })}
            </div>
            {move || match history.data.get() {
                None => view! { <div class="h-24 animate-pulse bg-white/60"></div> }.into_any(),
                Some((entries, _)) if entries.is_empty() => view! {
                    <p class="px-4 py-6 text-center text-sm text-(--color-muted)">"No changes recorded yet."</p>
                }.into_any(),
                Some((entries, _)) => view! { <HistoryList entries=entries categories=categories.data.get().unwrap_or_default() /> }.into_any(),
            }}
        </section>
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Named {
    id: String,
    code: String,
    name: String,
}

/// The calculator's History tab: who changed this product, when, and what.
#[component]
pub fn CostingHistory() -> impl IntoView {
    let params = use_params_map();
    let code = Memo::new(move |_| decode_code(&params.with(|p| p.get("code").unwrap_or_default())));
    view! {
        <div class="space-y-3">
            <crate::pages::calculator::Tabs master=false />
            {move || view! { <CostingHistoryOf code=code.get() /> }}
        </div>
    }
}

#[component]
fn CostingHistoryOf(code: String) -> impl IntoView {
    let key = format!("history-product:{code}");
    let product = live_with(key, move || {
        let code = code.clone();
        async move { Q::from("products").select("id,code,name").eq("code", &code).one::<Named>().await }
    });
    move || match product.data.get() {
        None => view! { <div class="card h-40 animate-pulse bg-white/60"></div> }.into_any(),
        Some(None) => view! { <div class="card p-6 text-sm">"No such product."</div> }.into_any(),
        Some(Some(p)) => view! {
            <p class="text-sm">
                <A href=product_href(&p.code) attr:class="font-mono font-semibold hover:underline">{p.code.clone()}</A>
                " " <span class="text-(--color-muted)">{p.name.clone()}</span>
            </p>
            <ProductHistory product_id=p.id shown=50 />
        }
        .into_any(),
    }
}
