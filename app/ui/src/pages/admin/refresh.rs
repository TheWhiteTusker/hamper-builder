//! Refresh hamper prices (port of `admin/refresh-prices/*`): saved hampers
//! keep their own copy of each product's cost and price; this pulls those
//! copies up to date. Final catalogue prices and quotations are untouched.

use crate::components::PageHeader;
use crate::db::{self, me};
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::lenient;
use lattice_core::money::format_money;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct PriceDiff {
    hamper_code: String,
    line_no: i64,
    product_code: Option<String>,
    product_name: Option<String>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    old_cp: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    new_cp: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    old_sp: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    new_sp: Option<Decimal>,
    note: String,
}

async fn refresh(apply: bool) -> Result<Vec<PriceDiff>, ApiError> {
    db::rpc("refresh_hamper_prices", &json!({ "p_apply": apply })).await
}

#[component]
pub fn RefreshPrices() -> impl IntoView {
    if !me().role.is_admin() {
        return view! { <div class="card p-6 text-sm text-(--color-muted)">"Only an admin can refresh hamper prices."</div> }.into_any();
    }
    let diffs = RwSignal::new(None::<Vec<PriceDiff>>);
    let error = RwSignal::new(None::<String>);
    let applied = RwSignal::new(None::<usize>);
    let busy = RwSignal::new(false);
    // Preview only: nothing is written until the button is used.
    let preview = move || {
        spawn_local(async move {
            match refresh(false).await {
                Ok(d) => diffs.set(Some(d)),
                Err(e) => {
                    error.set(Some(e.to_string()));
                    diffs.set(Some(Vec::new()));
                }
            }
        });
    };
    preview();
    let apply = move || {
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match refresh(true).await {
                Ok(d) => {
                    applied.set(Some(d.iter().filter(|x| x.note == "changed").count()));
                    // Hamper totals come from a view, so nothing else needs recalculating.
                    db::cache_drop(&["hampers", "dashboard", "quote_options"]).await;
                    preview();
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    };
    let money = |v: Option<Decimal>| format_money(v);
    view! {
        <PageHeader title="Refresh hamper prices" subtitle="Re-reads saved hampers against the current Product Master">""</PageHeader>
        <p class="card mb-4 p-4 text-sm text-(--color-muted)">
            "Saved hampers keep their own copy of each product’s cost and price, so changing the Product Master never moves them on its own. This is where you pull those copies up to date. Final catalogue prices are left exactly as approved, and quotations are not touched at all."
        </p>
        {move || applied.get().map(|n| view! {
            <p class="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">{format!("Updated {n} hamper line(s).")}</p>
        })}
        {move || error.get().map(|e| view! { <p role="alert" class="mb-4 text-sm text-red-700">{e}</p> })}
        {move || match diffs.get() {
            None => view! { <div class="card h-40 animate-pulse bg-white/60"></div> }.into_any(),
            Some(list) if list.is_empty() => view! {
                <div class="card p-8 text-center">
                    <p class="font-medium">"Everything is already up to date."</p>
                    <p class="mt-1 text-sm text-(--color-muted)">"No saved hamper differs from the Product Master."</p>
                </div>
            }.into_any(),
            Some(list) => {
                let changed = list.iter().filter(|d| d.note == "changed").count();
                let missing = list.iter().filter(|d| d.note == "missing").count();
                let hampers = list.iter().filter(|d| d.note == "changed").map(|d| d.hamper_code.clone()).collect::<HashSet<_>>().len();
                view! {
                    <div class="mb-4 flex flex-wrap items-center gap-3">
                        <p class="text-sm">
                            <strong>{changed}</strong>" line(s) across "<strong>{hampers}</strong>" hamper(s) would change."
                            {(missing > 0).then(|| format!(" {missing} line(s) reference a product that no longer exists."))}
                        </p>
                        {(changed > 0).then(|| view! {
                            <button type="button" class="btn-primary" disabled=move || busy.get() on:click=move |_| apply()>
                                {move || if busy.get() { "Updating…".to_string() } else { format!("Update {changed} line{}", if changed == 1 { "" } else { "s" }) }}
                            </button>
                        })}
                    </div>
                    <div class="card overflow-x-auto">
                        <table class="w-full min-w-[900px] border-collapse text-sm">
                            <thead><tr>
                                <th class="th">"Hamper"</th><th class="th">"Line"</th><th class="th">"Product"</th><th class="th num">"Cost now"</th>
                                <th class="th num">"New cost"</th><th class="th num">"Price now"</th><th class="th num">"New price"</th><th class="th"></th>
                            </tr></thead>
                            <tbody>
                                {list.into_iter().map(|d| {
                                    let gone = d.note == "missing";
                                    view! {
                                        <tr>
                                            <td class="td whitespace-nowrap font-mono">{d.hamper_code}</td>
                                            <td class="td text-(--color-muted)">{d.line_no}</td>
                                            <td class="td">{d.product_name.unwrap_or_default()}<span class="ml-2 font-mono text-xs text-(--color-muted)">{d.product_code.unwrap_or_default()}</span></td>
                                            <td class="td num text-(--color-muted)">{money(d.old_cp)}</td>
                                            <td class="td num font-medium">{if gone { "—".to_string() } else { money(d.new_cp) }}</td>
                                            <td class="td num text-(--color-muted)">{money(d.old_sp)}</td>
                                            <td class="td num font-medium">{if gone { "—".to_string() } else { money(d.new_sp) }}</td>
                                            <td class="td">{gone.then(|| view! { <span class="badge border-amber-300 bg-amber-50 text-amber-900">"not in Product Master"</span> })}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }
        }}
    }
    .into_any()
}
