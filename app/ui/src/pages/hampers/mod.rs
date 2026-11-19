//! Hampers: the list, a hamper's read-only page, and the builder
//! (ports of `(app)/hampers/*`).

mod builder;
mod image;

pub use builder::{EditHamper, NewHamper};
pub use image::HamperImage;
use crate::components::PageHeader;
use crate::data::use_settings;
use crate::db::{Q, live, live_with, me};
use crate::kit::{FallbackImg, ImagePreview, ListMenu, RowMenu, SelectBox, Selection, SelectionCount};
use lattice_core::api::ApiError;
use lattice_core::images::thumb_url;
use lattice_core::money::{format_money, format_pct};
use lattice_core::numbers::{parse_num, round2};
use lattice_core::records::{HamperItem, HamperSummary};
use lattice_core::Decimal;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use serde::{Deserialize, Serialize};

pub fn hamper_href(code: &str) -> String {
    format!("/hampers/{}", String::from(js_sys::encode_uri_component(code)))
}

fn summaries() -> Q {
    Q::from("hamper_summary").select("*").order("code")
}

/// A number filter from the URL; blank means no bound.
fn bound(v: &str) -> Option<Decimal> {
    (!v.trim().is_empty()).then(|| parse_num(v))
}

#[component]
pub fn Hampers() -> impl IntoView {
    let profile = me();
    let manage = profile.role.can_manage();
    let admin = profile.role.is_admin();
    let hampers = live::<HamperSummary>("hampers", summaries());
    let settings = use_settings();
    let sel = Selection::new("hampers");
    let query = use_query_map();
    let navigate = StoredValue::new_local(use_navigate());
    let param = move |k: &str| query.with(|m| m.get(k).unwrap_or_default());
    let set_param = move |k: &'static str, v: String| {
        let mut m = query.get_untracked();
        if v.is_empty() {
            m.remove(k);
        } else {
            m.replace(k, v);
        }
        let qs = m.to_query_string();
        navigate.with_value(|nav| nav(&format!("/hampers{qs}"), NavigateOptions { replace: true, scroll: false, ..Default::default() }));
    };
    let keys = ["q", "collection", "items", "cost_min", "cost_max", "price_min", "price_max"];
    let filtered_on = move || keys.iter().any(|k| !param(k).trim().is_empty());

    // Filtered on the device: every keystroke narrows the list at once.
    let rows = Memo::new(move |_| {
        let q = param("q").trim().to_lowercase();
        let collection = param("collection");
        let (items, cmin, cmax, pmin, pmax) =
            (bound(&param("items")), bound(&param("cost_min")), bound(&param("cost_max")), bound(&param("price_min")), bound(&param("price_max")));
        hampers.data.with(|all| {
            all.as_ref().map(|all| {
                all.iter()
                    .filter(|h| q.is_empty() || h.name.to_lowercase().contains(&q) || h.code.to_lowercase().contains(&q))
                    .filter(|h| collection.is_empty() || h.collection.as_deref() == Some(collection.as_str()))
                    .filter(|h| items.is_none_or(|n| h.number_of_items >= n))
                    .filter(|h| cmin.is_none_or(|n| h.total_cp >= n) && cmax.is_none_or(|n| h.total_cp <= n))
                    .filter(|h| pmin.is_none_or(|n| h.final_catalogue_sp.is_some_and(|p| p >= n)))
                    .filter(|h| pmax.is_none_or(|n| h.final_catalogue_sp.is_some_and(|p| p <= n)))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    });
    let subtitle = move || {
        let n = rows.with(|r| r.as_ref().map_or(0, Vec::len));
        format!("{n} hamper{}", if n == 1 { "" } else { "s" })
    };
    let deleted = move || param("deleted") == "1";
    let field = move |k: &'static str, label: &'static str, class: &'static str| {
        view! {
            <input aria-label=label placeholder=if k.ends_with("max") { "max" } else if k.ends_with("min") || k == "items" { "min" } else { "" } inputmode="decimal"
                class=class prop:value=move || param(k) on:input=move |e| set_param(k, event_target_value(&e)) />
        }
    };

    view! {
        <PageHeader title="Hampers" subtitle=Signal::derive(subtitle)>
            {admin.then(|| view! { <A href="/admin/refresh-prices" attr:class="btn-secondary">"Refresh prices"</A> })}
            {manage.then(|| view! { <A href="/hampers/new" attr:class="btn-primary">"New hamper"</A> })}
            <ListMenu sel=sel />
        </PageHeader>
        <Show when=deleted>
            <div class="mb-4 flex items-center justify-between rounded-lg border border-(--color-line) bg-[#faf8ee] px-4 py-3 text-sm text-(--color-ink) shadow-xs">
                <div class="flex items-center gap-2">
                    <span class="inline-block h-2 w-2 rounded-full bg-emerald-600"></span>
                    <span>"Hamper moved to Bin. It will remain in the Bin for 30 days and can be restored anytime."</span>
                </div>
                <A href="/bin?tab=hampers" attr:class="font-semibold text-(--color-brand) underline hover:text-(--color-brand-dark)">"Open Bin →"</A>
            </div>
        </Show>
        <SelectionCount sel=sel />
        <div class="card overflow-x-auto">
            <table class="w-full min-w-[1200px] border-collapse text-sm">
                <thead>
                    <tr>
                        <th class="th w-8" aria-label="Selected"></th>
                        <th class="th w-14"></th>
                        <th class="th">"Code"</th>
                        <th class="th">"Hamper"</th>
                        <th class="th">"Collection"</th>
                        <th class="th">"Status"</th>
                        <th class="th num">"Items"</th>
                        <th class="th num">"Cost"</th>
                        <th class="th num">"Catalogue price"</th>
                        <th class="th num">"Profit"</th>
                        <th class="th num">"Margin"</th>
                        <th class="th w-10"></th>
                    </tr>
                    <tr>
                        <th class="th pb-2" colspan="2"></th>
                        <th class="th pb-2" colspan="2">
                            <input aria-label="Search name or code" placeholder="Name or code" class="input" prop:value=move || param("q") on:input=move |e| set_param("q", event_target_value(&e)) />
                        </th>
                        <th class="th pb-2">
                            <input aria-label="Filter by collection" list="collection-options" placeholder="Any" class="input" prop:value=move || param("collection") on:input=move |e| set_param("collection", event_target_value(&e)) />
                            <datalist id="collection-options">
                                {move || settings.data.get().map(|s| s.collections.into_iter().map(|c| view! { <option value=c></option> }).collect_view())}
                            </datalist>
                        </th>
                        <th class="th pb-2"></th>
                        <th class="th pb-2">{field("items", "Minimum items", "input input-num")}</th>
                        <th class="th pb-2">
                            <div class="flex items-center gap-1">{field("cost_min", "Minimum cost", "input input-num")}<span class="text-(--color-muted)">"–"</span>{field("cost_max", "Maximum cost", "input input-num")}</div>
                        </th>
                        <th class="th pb-2">
                            <div class="flex items-center gap-1">{field("price_min", "Minimum catalogue price", "input input-num")}<span class="text-(--color-muted)">"–"</span>{field("price_max", "Maximum catalogue price", "input input-num")}</div>
                        </th>
                        <th class="th pb-2" colspan="3">
                            <div class="flex items-center justify-end gap-2">
                                <Show when=filtered_on>
                                    <A href="/hampers" attr:class="btn-secondary">"Clear"</A>
                                </Show>
                            </div>
                        </th>
                    </tr>
                </thead>
                <tbody>
                    {move || match rows.get() {
                        None => view! { <tr><td colspan="12" class="td h-40 animate-pulse bg-white/60"></td></tr> }.into_any(),
                        Some(list) if list.is_empty() => view! {
                            <tr><td colspan="12" class="td py-10 text-center text-sm text-(--color-muted)">
                                {if filtered_on() { "No hampers match that search." } else { "No hampers yet." }}
                                {(manage && !filtered_on()).then(|| view! { <A href="/hampers/new" attr:class="ml-2 underline">"Build the first one"</A> })}
                            </td></tr>
                        }.into_any(),
                        Some(list) => list.into_iter().map(|h| view! { <Row h=h sel=sel /> }).collect_view().into_any(),
                    }}
                </tbody>
            </table>
        </div>
    }
}

fn tone(v: Option<Decimal>) -> &'static str {
    if v.is_some_and(|v| v < Decimal::ZERO) { "text-red-700" } else { "" }
}

#[component]
fn Row(h: HamperSummary, sel: Selection) -> impl IntoView {
    let href = hamper_href(&h.code);
    let (id, sid) = (h.id.clone(), h.id.clone());
    let (code, label1, label2) = (h.code.clone(), h.code.clone(), h.code.clone());
    view! {
        <tr class=move || if sel.has(&sid) { "bg-(--color-brand)/5" } else { "" }>
            <td class="td w-8 px-2 text-center"><SelectBox sel=sel id=id.clone() label=label1 /></td>
            <td class="td">
                <A href=href.clone() attr:class="block h-10 w-10">
                    {match h.image_url.clone() {
                        Some(url) => view! { <img src=url alt="" loading="lazy" class="h-10 w-10 rounded-md border border-(--color-line) object-cover" /> }.into_any(),
                        None => view! { <span class="block h-10 w-10 rounded-md border border-dashed border-(--color-line) bg-(--color-paper)"></span> }.into_any(),
                    }}
                </A>
            </td>
            <td class="td whitespace-nowrap font-mono"><A href=href.clone() attr:class="hover:underline">{code}</A></td>
            <td class="td"><A href=href attr:class="hover:underline">{h.name.clone()}</A></td>
            <td class="td text-(--color-muted)">{h.collection.clone().unwrap_or_else(|| "—".into())}</td>
            <td class="td"><span class="badge">{h.status.clone()}</span></td>
            <td class="td num">{h.number_of_items.normalize().to_string()}</td>
            <td class="td num">{format_money(Some(h.total_cp))}</td>
            <td class="td num">
                {match h.final_catalogue_sp {
                    None => view! { <span class="text-(--color-muted)">"not priced"</span> }.into_any(),
                    Some(p) => format_money(Some(p)).into_any(),
                }}
            </td>
            <td class=format!("td num {}", tone(h.gross_profit))>{h.gross_profit.map_or_else(|| "—".into(), |g| format_money(Some(g)))}</td>
            <td class=format!("td num {}", tone(h.final_margin))>{h.final_margin.map_or_else(|| "—".into(), |m| format_pct(Some(m), 1))}</td>
            <td class="td w-10"><div class="flex justify-end"><RowMenu sel=sel id=id label=label2 /></div></td>
        </tr>
    }
}

/// A saved line plus its product's photo, if the product still exists.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreviewItem {
    #[serde(flatten)]
    pub item: HamperItem,
    pub products: Option<ImageRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageRef {
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct HamperPage {
    hamper: HamperSummary,
    items: Vec<PreviewItem>,
}

async fn load_hamper_page(code: String) -> Result<HamperPage, ApiError> {
    let hamper = summaries().eq("code", &code).one::<HamperSummary>().await?.ok_or_else(|| ApiError::NotFound("No such hamper.".into()))?;
    let items = Q::from("hamper_items").select("*,products(image_url)").eq("hamper_id", &hamper.id).order("line_no").rows().await?;
    Ok(HamperPage { hamper, items })
}

/// `/hampers/H128`: a saved hamper, read-only; Edit opens the builder.
#[component]
pub fn HamperView() -> impl IntoView {
    let params = use_params_map();
    let code = Memo::new(move |_| params.with(|p| p.get("code").map(|c| js_sys::decode_uri_component(&c).map(String::from).unwrap_or(c)).unwrap_or_default()));
    view! {
        {move || {
            let code = code.get();
            let c = code.clone();
            let page = live_with(format!("hamper:{code}"), move || load_hamper_page(c.clone()));
            let manage = me().role.can_manage();
            view! {
                {move || match (page.data.get(), page.error.get()) {
                    (None, Some(e)) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                    (None, None) => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
                    (Some(p), _) => view! { <HamperPageView page=p manage=manage /> }.into_any(),
                }}
            }
        }}
    }
}

#[component]
fn HamperPageView(page: HamperPage, manage: bool) -> impl IntoView {
    let h = page.hamper;
    let fields = [
        ("Hamper code", h.code.clone()),
        ("Collection", h.collection.clone().unwrap_or_else(|| "—".into())),
        ("Status", h.status.clone()),
        ("No. of items", round2(h.number_of_items).normalize().to_string()),
    ];
    let edit = format!("{}/edit", hamper_href(&h.code));
    let money = |v: Option<Decimal>| v.map_or_else(|| "—".into(), |v| format_money(Some(v)));
    let good_bad = |v: Option<Decimal>| match v {
        None => "text-(--color-ink)",
        Some(v) if v < Decimal::ZERO => "text-red-700",
        Some(_) => "text-emerald-700",
    };
    let boxes = [
        ("Total cost", format_money(Some(h.total_cp)), "text-(--color-ink)"),
        ("Base SP", format_money(Some(h.base_sp)), "text-(--color-ink)"),
        ("Discount", format!("{}%", round2(h.discount_pct * Decimal::from(100)).normalize()), "text-(--color-ink)"),
        ("SP after discount", format_money(Some(h.sp_after_discount)), "text-(--color-ink)"),
        ("Target SP", money(h.target_sp), "text-(--color-ink)"),
        ("Variance", money(h.variance), good_bad(h.variance)),
        ("Final catalogue SP", money(h.final_catalogue_sp), "text-(--color-brand-dark)"),
        ("Final margin", h.final_margin.map_or_else(|| "—".into(), |m| format_pct(Some(m), 1)), good_bad(h.final_margin)),
    ];
    let count = page.items.len();
    view! {
        <PageHeader title=h.name.clone() subtitle=h.code.clone()>
            <span class="badge">{h.status.clone()}</span>
            {manage.then(|| view! { <A href=edit attr:class="btn-primary">"Edit"</A> })}
        </PageHeader>
        <div class="space-y-6">
            <div class="card flex flex-wrap gap-5 p-5">
                {h.image_url.clone().map(|url| view! {
                    <ImagePreview src=url alt=format!("{} image", h.name) class="h-32 w-32 shrink-0 rounded-lg border border-slate-200 bg-slate-100" />
                })}
                <div class="grid flex-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
                    {fields.into_iter().map(|(label, value)| view! {
                        <div><div class="label">{label}</div><div class="mt-1 text-sm text-(--color-ink)">{value}</div></div>
                    }).collect_view()}
                    {h.notes.clone().filter(|n| !n.is_empty()).map(|n| view! {
                        <div class="sm:col-span-2 lg:col-span-4"><div class="label">"Notes"</div><p class="mt-1 whitespace-pre-line text-sm text-(--color-ink)">{n}</p></div>
                    })}
                </div>
            </div>
            <section class="card overflow-hidden">
                <div class="border-b border-(--color-line) px-4 py-2.5">
                    <h2 class="text-sm font-semibold">"Products"<span class="ml-2 font-normal text-(--color-muted)">{format!("{count} line{}", if count == 1 { "" } else { "s" })}</span></h2>
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full min-w-240 border-collapse text-sm">
                        <thead><tr>
                            <th class="th">"Product"</th><th class="th">"Code"</th><th class="th num">"Qty"</th><th class="th num">"Unit cost"</th>
                            <th class="th num">"Total cost"</th><th class="th num">"Margin %"</th><th class="th num">"Unit price"</th><th class="th num">"Total price"</th>
                        </tr></thead>
                        <tbody>
                            {page.items.into_iter().map(|p| {
                                let i = p.item;
                                view! {
                                    <tr>
                                        <td class="td">
                                            <div class="flex items-center gap-2">
                                                {p.products.and_then(|x| x.image_url).map(|url| view! {
                                                    <div class="relative h-8 w-8 shrink-0 overflow-hidden rounded-md border border-slate-200 bg-slate-100">
                                                        <FallbackImg src=thumb_url(&url) fallback=url alt="" class="h-full w-full object-cover" />
                                                    </div>
                                                })}
                                                <div>
                                                    <div class="font-medium">{i.product_name.clone()}</div>
                                                    {i.category_name.clone().filter(|c| !c.is_empty()).map(|c| view! { <div class="text-xs text-(--color-muted)">{c}</div> })}
                                                </div>
                                            </div>
                                        </td>
                                        <td class="td font-mono text-xs text-(--color-muted)">{i.product_code.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| "—".into())}</td>
                                        <td class="td num">{i.qty.normalize().to_string()}</td>
                                        <td class="td num">{format_money(Some(i.unit_cp))}</td>
                                        <td class="td num text-(--color-muted)">{format_money(Some(i.total_cp))}</td>
                                        <td class="td num">{format_pct(Some(i.target_margin), 1)}</td>
                                        <td class="td num">{format_money(Some(i.unit_sp))}</td>
                                        <td class="td num font-medium">{format_money(Some(i.total_sp))}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </section>
            <div class="card border-2 border-(--color-brand)/20 bg-gradient-to-br from-white to-slate-50 p-5 shadow-md">
                <h3 class="mb-4 text-base font-bold text-(--color-ink)">"Hamper pricing"</h3>
                <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                    {boxes.into_iter().map(|(label, value, color)| view! {
                        <div class="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
                            <span class="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">{label}</span>
                            <div class=format!("mt-1 font-mono text-2xl font-black {color}")>{value}</div>
                        </div>
                    }).collect_view()}
                </div>
                {h.gross_profit.map(|g| view! {
                    <p class="mt-3 text-sm text-(--color-muted)">"Gross profit "<span class=format!("font-mono font-semibold {}", good_bad(Some(g)))>{format_money(Some(g))}</span></p>
                })}
            </div>
        </div>
    }
}
