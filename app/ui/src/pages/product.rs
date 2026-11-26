//! One product's saved costing, read-only (port of `products/[...code]/page.tsx`
//! and `_view/*`). Edit opens the cost calculator.

use crate::components::PageHeader;
use crate::data::{ProductCosting, load_product_costing, use_categories, use_cost_rows, use_settings};
use crate::db::live_with;
use crate::kit::{ImagePreview, costing_href, decode_code, product_href};
use lattice_core::colors::{ProductColor, resolve_colors};
use lattice_core::cost_tree::build_cost_tree;
use lattice_core::costing::{
    CostSheetTotals, ProductCostLine, calculate_cost_sheet_totals, is_sized_unit, is_time_unit, with_overhead,
};
use lattice_core::js;
use lattice_core::money::{format_money, format_pct};
use lattice_core::mrp::mrp_limit;
use lattice_core::product_code::{codes_for_colors, parse_product_code};
use lattice_core::{Decimal, api::ApiError};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde_json::json;

fn n(d: Decimal) -> String {
    d.normalize().to_string()
}

#[component]
pub fn ProductView() -> impl IntoView {
    let params = use_params_map();
    let code = Memo::new(move |_| decode_code(&params.with(|p| p.get("code").unwrap_or_default())));
    view! { {move || { let code = code.get(); view! { <ProductPage code=code /> } }} }
}

#[component]
fn ProductPage(code: String) -> impl IntoView {
    let key = format!("product:{code}");
    let c = code.clone();
    let costing = live_with(key, move || {
        let c = c.clone();
        async move {
            load_product_costing(&c).await.and_then(|p| p.ok_or_else(|| ApiError::NotFound("No such product.".into())))
        }
    });
    let rows = use_cost_rows();
    let settings = use_settings();
    let categories = use_categories();

    view! {
        <crate::list_position::BackToList kind="products" />
        {move || match (costing.data.get(), costing.error.get()) {
            (None, Some(e)) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
            (None, None) => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
            (Some(pc), _) => {
                let stages = rows.data.get().map(|r| build_cost_tree(&r)).unwrap_or_default();
                let s = settings.data.get().unwrap_or_default();
                let category = categories
                    .data
                    .get()
                    .unwrap_or_default()
                    .into_iter()
                    .find(|c| Some(&c.id) == pc.product.category_id.as_ref())
                    .map(|c| c.name);
                let palette = resolve_colors(&s.product_colors, &s.color_hex.clone().into_iter().collect());
                let colors = resolve_colors(&pc.product.colors, &s.color_hex.clone().into_iter().collect());
                view! { <Loaded pc=pc stages=stages gst=s.gst_rate category=category palette=palette colors=colors /> }.into_any()
            }
        }}
    }
}

#[component]
fn Loaded(
    pc: ProductCosting,
    stages: Vec<lattice_core::cost_tree::StageNode>,
    gst: Decimal,
    category: Option<String>,
    palette: Vec<ProductColor>,
    colors: Vec<ProductColor>,
) -> impl IntoView {
    let ProductCosting { product, own_color, sheet, images, .. } = pc;
    let lines = sheet.as_ref().map(|s| s.lines.clone()).unwrap_or_default();
    let overheads = sheet.as_ref().map(|s| s.sheet.stage_overheads.clone()).unwrap_or_default();
    let totals = sheet.as_ref().map(|s| calculate_cost_sheet_totals(&lines, s.sheet.markup_pct, &overheads));
    let mrp_max = totals.as_ref().and_then(|t| mrp_limit(&lines, Some(&overheads), t.total_cost, gst));
    // Cost types in the calculator's order, skipping any with no lines; the rest is bought out.
    let sections: Vec<_> = stages.iter().filter(|s| lines.iter().any(|l| l.stage_code == s.row.code)).collect();
    let known: Vec<&str> = stages.iter().map(|s| s.row.code.as_str()).collect();
    let bought: Vec<ProductCostLine> =
        lines.iter().filter(|l| !known.contains(&l.stage_code.as_str())).cloned().collect();
    let markup = sheet.as_ref().map(|s| s.sheet.markup_pct).or(product.markup_pct);
    let notes = sheet.as_ref().and_then(|s| s.sheet.notes.clone());

    let edit_href = costing_href(&product.code);
    let (title, subtitle) = (product.name.clone(), product.code.clone());
    view! {
        <PageHeader title=title subtitle=subtitle>
            <A href=edit_href attr:class="btn-secondary">"Edit"</A>
            <A href="/products/new" attr:class="btn-primary">"New product"</A>
        </PageHeader>
        <div class="space-y-6">
            <ChangeColor id=product.id.clone() code=product.code.clone() current=own_color palette=palette />
            <ProductCard product=product.clone() category=category colors=colors photos=images />
            {sheet.is_none().then(|| view! {
                <div class="card border-dashed p-4 text-center text-sm text-(--color-muted)">
                    "No costing saved for this product yet. Click " <strong>"Edit"</strong> " to cost it in the calculator."
                </div>
            })}
            {sections
                .into_iter()
                .map(|s| {
                    let stage_lines: Vec<ProductCostLine> = lines.iter().filter(|l| l.stage_code == s.row.code).cloned().collect();
                    let oh = overheads.get(&s.row.code.to_lowercase()).copied().unwrap_or_default();
                    view! { <CostTable badge=s.row.sort_order.to_string() title=s.row.name.clone() lines=stage_lines overhead=oh /> }
                })
                .collect_view()}
            {(!bought.is_empty()).then(|| view! { <CostTable badge="B".into() title="Bought Out Items".into() lines=bought overhead=Decimal::ZERO /> })}
            <PriceSummary totals=totals cost=product.cost_price markup=markup sp=product.default_sp mrp_max=mrp_max notes=notes />
            {crate::db::me().role.is_admin().then(|| view! { <crate::pages::history::ProductHistory product_id=product.id.clone() /> })}
        </div>
    }
}

/// Changes this one product's colour; the code's last part, cost sheet, hamper
/// lines and photo tags follow it (`recolor_product`). Other colours are untouched.
#[component]
fn ChangeColor(id: String, code: String, current: String, palette: Vec<ProductColor>) -> impl IntoView {
    let color = RwSignal::new(current.clone());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let navigate = use_navigate();
    let parsed = parse_product_code(&code);
    let pal = palette.clone();
    let next = move || pal.iter().find(|c| c.name == color.get()).cloned();
    let next_code = {
        let (code, next) = (code.clone(), next.clone());
        move || match next() {
            Some(n) if !parsed.category_code.is_empty() => {
                format!("{}/{}/{}", parsed.category_code, parsed.serial, n.code)
            }
            _ => code.clone(),
        }
    };
    let (cur, code2) = (current.clone(), code.clone());
    let submit = {
        let (next_code, next) = (next_code.clone(), next.clone());
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let Some(c) = next() else {
                error.set(Some("Pick a colour.".into()));
                return;
            };
            busy.set(true);
            let (id, navigate, _target) = (id.clone(), navigate.clone(), next_code());
            spawn_local(async move {
                let res: Result<String, _> = crate::db::rpc(
                    "recolor_product",
                    &json!({ "p_id": id, "p_color": c.name, "p_color_code": c.code }),
                )
                .await;
                busy.set(false);
                match res {
                    Ok(new_code) => {
                        crate::db::cache_drop(&["products", "catalog"]).await;
                        navigate(&product_href(&new_code), Default::default());
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        }
    };
    view! {
        <form on:submit=submit class="flex flex-wrap items-center gap-2">
            <label for="change-color" class="text-xs font-semibold text-(--color-ink)">"Colour"</label>
            <select id="change-color" class="select w-auto py-1 text-xs" on:change=move |e| color.set(event_target_value(&e))>
                {current.is_empty().then(|| view! { <option value="" selected=true>"Not set"</option> })}
                {palette
                    .into_iter()
                    .map(|c| {
                        let selected = c.name == current;
                        view! { <option value=c.name.clone() selected=selected>{format!("{} ({})", c.name, c.code)}</option> }
                    })
                    .collect_view()}
            </select>
            {move || {
                (color.get() != cur && next().is_some()).then(|| {
                    let label = if busy.get() {
                        "Changing…".to_string()
                    } else if next_code() != code2 {
                        format!("Change to {}", next_code())
                    } else {
                        "Save colour".to_string()
                    };
                    view! { <button type="submit" disabled=move || busy.get() class="btn-secondary py-1 text-xs">{label}</button> }
                })
            }}
            {move || error.get().map(|e| view! { <span role="alert" class="text-xs text-red-700">{e}</span> })}
        </form>
    }
}

#[component]
fn ProductCard(
    product: lattice_core::records::Product,
    category: Option<String>,
    colors: Vec<ProductColor>,
    photos: Vec<lattice_core::images::ProductImage>,
) -> impl IntoView {
    let fields = [
        ("Product code", view! { <span class="font-mono font-semibold">{product.code.clone()}</span> }.into_any()),
        ("Category", category.unwrap_or_else(|| "—".into()).into_any()),
        ("Product name", product.name.clone().into_any()),
        ("Product origin", product.source.clone().unwrap_or_else(|| "—".into()).into_any()),
    ];
    let code = product.code.clone();
    let name = product.name.clone();
    view! {
        <div class="card p-5">
            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {fields
                    .into_iter()
                    .map(|(label, value)| view! {
                        <div>
                            <div class="label">{label}</div>
                            <div class="mt-1 text-sm text-(--color-ink)">{value}</div>
                        </div>
                    })
                    .collect_view()}
            </div>
            <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-(--color-border) pt-4">
                <div class="flex flex-wrap items-center gap-2">
                    <span class="text-xs font-semibold text-(--color-ink)">"Color finishes:"</span>
                    {colors.is_empty().then(|| view! { <span class="text-xs text-(--color-muted)">"—"</span> })}
                    {colors
                        .into_iter()
                        .map(|c| {
                            let sibling = codes_for_colors(&code, &[c.name.as_str()]).into_iter().next().map_or_else(|| code.clone(), |s| s.code);
                            let dot = format!("background-color: {}", c.hex);
                            let text = format!("{} ({})", c.name, c.code);
                            if sibling == code {
                                view! {
                                    <span class="flex items-center gap-1.5 rounded-full bg-(--color-brand) px-2.5 py-1 text-xs font-bold text-white shadow-xs">
                                        <span class="h-2.5 w-2.5 rounded-full border border-white/40" style=dot></span>
                                        {text}
                                    </span>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <A
                                        href=product_href(&sibling)
                                        attr:title=format!("View {} variant", c.name)
                                        attr:class="flex items-center gap-1.5 rounded-full bg-(--color-sheet) px-2.5 py-1 text-xs font-medium transition-colors hover:bg-slate-200"
                                    >
                                        <span class="h-2.5 w-2.5 rounded-full border border-black/20" style=dot></span>
                                        {text}
                                    </A>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()}
                </div>
                <span class=format!("badge {}", if product.is_active { "" } else { "opacity-70" })>
                    {if product.is_active { "Active in Product Master & Hamper Builder" } else { "Inactive" }}
                </span>
            </div>
            {(!photos.is_empty()).then(|| view! {
                <div class="mt-4 flex flex-wrap gap-3 border-t border-(--color-border) pt-4">
                    {photos
                        .into_iter()
                        .map(|img| {
                            let alt = match img.color.as_deref() {
                                Some(c) if !c.is_empty() => format!("{name} — {c}"),
                                _ => name.clone(),
                            };
                            let caption = format!("{}{}", img.color.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| "General".into()), if img.is_primary { " · ★" } else { "" });
                            view! {
                                <figure class="w-24">
                                    <ImagePreview src=img.url alt=alt class="aspect-square w-full rounded-lg border border-slate-200 bg-slate-100" />
                                    <figcaption class="mt-1 truncate text-[11px] text-(--color-muted)">{caption}</figcaption>
                                </figure>
                            }
                        })
                        .collect_view()}
                </div>
            })}
        </div>
    }
}

/// What was measured: the size for sized lines, minutes for machine time.
fn size(l: &ProductCostLine) -> String {
    let unit = if l.unit.is_empty() { "min" } else { l.unit.as_str() };
    if l.stage_code == "machine" && is_time_unit(Some(unit)) {
        return format!("{} min", n(l.duration_minutes.unwrap_or_default()));
    }
    if !is_sized_unit(Some(&l.unit)) {
        return "—".into();
    }
    let u = match l.dimension_unit.as_deref().unwrap_or("inch") {
        "inch" => "″",
        "mm" => " mm",
        "cm" => " cm",
        _ => "",
    };
    let (len, br) = (l.length.unwrap_or_default(), l.breadth.unwrap_or_default());
    if len > Decimal::ZERO && br > Decimal::ZERO {
        format!("{}{u} × {}{u}", n(len), n(br))
    } else if len > Decimal::ZERO {
        format!("{}{u}", n(len))
    } else {
        "—".into()
    }
}

/// One cost type as saved: its total and any overhead %, and its lines.
#[component]
pub fn CostTable(badge: String, title: String, lines: Vec<ProductCostLine>, overhead: Decimal) -> impl IntoView {
    let lines_total = js::dec(lines.iter().fold(0.0, |s, l| s + js::f(l.line_total)));
    let total = with_overhead(lines_total, overhead);
    let count = lines.len();
    view! {
        <div class="card overflow-hidden shadow-sm">
            <div class="flex flex-wrap items-center justify-between gap-2 border-b border-(--color-border) bg-(--color-sheet) px-4 py-3">
                <div class="flex items-center gap-3">
                    <span class="flex h-7 w-7 items-center justify-center rounded-full bg-(--color-brand) text-xs font-bold text-white">{badge}</span>
                    <h3 class="text-base font-semibold text-(--color-ink)">{title}</h3>
                    <span class="text-xs text-(--color-muted)">{format!("{count} line{}", if count == 1 { "" } else { "s" })}</span>
                </div>
                <div class="flex items-center gap-3 text-xs text-(--color-muted)">
                    {(overhead > Decimal::ZERO).then(|| view! {
                        <span>
                            {format!("Overhead {}% ", n(overhead))}
                            <span class="font-mono text-(--color-brand-dark)">{format!("+{}", format_money(Some(js::dec(js::f(total) - js::f(lines_total)))))}</span>
                        </span>
                    })}
                    <span>"Total:"</span>
                    <span class="rounded-lg bg-white px-2.5 py-1 font-mono text-sm font-bold text-(--color-brand-dark) shadow-sm">{format_money(Some(total))}</span>
                </div>
            </div>
            <div class="overflow-x-auto p-2">
                <table class="w-full text-left text-xs">
                    <thead>
                        <tr class="border-b border-(--color-border) font-semibold text-(--color-muted)">
                            <th class="min-w-60 p-2">"Item"</th>
                            <th class="min-w-27.5 p-2">"Size / Time"</th>
                            <th class="min-w-27.5 p-2">"Rate / Unit"</th>
                            <th class="p-2 text-right">"Qty"</th>
                            <th class="p-2 text-right">"Wastage"</th>
                            <th class="min-w-23.75 p-2 text-right">"Total Cost"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-(--color-border)">
                        {lines
                            .into_iter()
                            .map(|l| {
                                let title = l.variety_name.clone().filter(|v| !v.is_empty()).or_else(|| Some(l.item_name.clone()).filter(|v| !v.is_empty())).unwrap_or_else(|| "—".into());
                                let path = [Some(l.category_name.clone()), l.subcategory_name.clone()]
                                    .into_iter()
                                    .flatten()
                                    .filter(|s| !s.is_empty())
                                    .collect::<Vec<_>>()
                                    .join(" › ");
                                let has_variety = l.variety_name.as_deref().is_some_and(|v| !v.is_empty());
                                let mrp = l.mrp.filter(|m| *m > Decimal::ZERO);
                                view! {
                                    <tr>
                                        <td class="p-2">
                                            <div class="font-semibold text-(--color-ink)">{title}</div>
                                            {(has_variety && !path.is_empty()).then(|| view! { <div class="text-[11px] text-(--color-muted)">{path}</div> })}
                                        </td>
                                        <td class="p-2 text-(--color-muted)">{size(&l)}</td>
                                        <td class="whitespace-nowrap p-2 font-mono">
                                            {format!("{}/{}", format_money(Some(l.rate)), l.unit)}
                                            {mrp.map(|m| view! { <div class="text-[11px] text-(--color-muted)">{format!("MRP {}", format_money(Some(m)))}</div> })}
                                        </td>
                                        <td class="p-2 text-right tabular-nums">{n(l.qty_or_one())}</td>
                                        <td class="p-2 text-right tabular-nums text-(--color-muted)">
                                            {if l.wastage_pct.is_zero() { "—".to_string() } else { format!("{}%", n(l.wastage_pct)) }}
                                        </td>
                                        <td class="p-2 text-right font-mono font-semibold text-(--color-ink)">{format_money(Some(l.line_total))}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Stage share of the cost price, as one stacked bar with a legend.
#[component]
pub fn CostBreakdown(totals: CostSheetTotals) -> impl IntoView {
    let total = js::f(totals.total_cost);
    (total > 0.0).then(|| {
        let parts = [
            ("Material", totals.material_total, "bg-amber-600", false),
            ("Hardware", totals.hardware_total, "bg-blue-600", false),
            ("Finishing", totals.finishing_total, "bg-purple-600", false),
            ("Machine", totals.machine_total, "bg-teal-600", false),
            ("Misc & Bought Out", totals.other_total, "bg-amber-400", true),
        ];
        view! {
            <div class="mb-6">
                <div class="flex h-3 w-full overflow-hidden rounded-full bg-slate-100 shadow-inner">
                    {parts
                        .iter()
                        .map(|(label, v, color, _)| {
                            let w = format!("width: {}%", js::f(*v) / total * 100.0);
                            view! { <div style=w class=*color title=format!("{label}: {}", format_money(Some(*v)))></div> }
                        })
                        .collect_view()}
                </div>
                <div class="mt-2 flex flex-wrap gap-4 text-xs">
                    {parts
                        .iter()
                        .filter(|(_, v, _, optional)| !optional || *v > Decimal::ZERO)
                        .map(|(label, v, color, _)| {
                            let share = format_pct(Some(js::dec(js::f(*v) / total)), 0);
                            view! {
                                <span class="flex items-center gap-1.5">
                                    <span class=format!("h-2.5 w-2.5 rounded-full {color}")></span>
                                    {format!("{label}: {} ({share})", format_money(Some(*v)))}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}

#[component]
fn PriceSummary(
    totals: Option<CostSheetTotals>,
    cost: Decimal,
    markup: Option<Decimal>,
    sp: Decimal,
    mrp_max: Option<Decimal>,
    notes: Option<String>,
) -> impl IntoView {
    let (spf, cpf) = (js::f(sp), js::f(cost));
    let boxes = [
        ("Total Cost Price (CP)", format_money(Some(cost)), "text-(--color-ink)"),
        ("Markup %", markup.map_or_else(|| "—".into(), |m| format!("{}%", n(m))), "text-(--color-ink)"),
        ("Selling Price (SP)", format_money(Some(sp)), "text-(--color-brand-dark)"),
        (
            "Margin",
            if spf > 0.0 { format_pct(Some(js::dec((spf - cpf) / spf)), 1) } else { "—".into() },
            "text-emerald-700",
        ),
    ];
    view! {
        <div class="card border-2 border-(--color-brand)/20 bg-gradient-to-br from-white to-slate-50 p-5 shadow-md">
            <h3 class="mb-4 text-base font-bold text-(--color-ink)">"Cost Breakdown & Selling Price"</h3>
            {totals.map(|t| view! { <CostBreakdown totals=t /> })}
            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {boxes
                    .into_iter()
                    .map(|(label, value, tone)| view! {
                        <div class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
                            <span class="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">{label}</span>
                            <div class=format!("mt-1 font-mono text-2xl font-black {tone}")>{value}</div>
                        </div>
                    })
                    .collect_view()}
            </div>
            {mrp_max.map(|m| {
                let over = sp > m;
                view! {
                    <p class=format!("mt-2 text-xs {}", if over { "font-semibold text-red-700" } else { "text-(--color-muted)" })>
                        {format!("{}: max SP {} (MRP includes GST)", if over { "Above MRP limit" } else { "MRP limit" }, format_money(Some(m)))}
                    </p>
                }
            })}
            {notes.filter(|n| !n.is_empty()).map(|notes| view! {
                <div class="mt-4">
                    <div class="label">"Costing notes / specifications"</div>
                    <p class="mt-1 whitespace-pre-line text-sm text-(--color-ink)">{notes}</p>
                </div>
            })}
        </div>
    }
}
