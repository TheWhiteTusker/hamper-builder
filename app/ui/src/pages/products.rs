//! Product Master list (port of `(app)/products/page.tsx` and `_list/*`).
//! The whole list is cached on the device and filtered locally, so typing
//! filters instantly with no round trip.

use crate::components::{EmptyState, PageHeader};
use crate::data::use_categories;
use crate::db::{self, Q, live, me};
use crate::kit::{
    ImagePreview, ListMenu, Menu, MenuItem, ROW_TINT, RowNumber, Selection, SelectionCount, confirm, costing_href,
    product_href,
};
use lattice_core::money::format_money;
use lattice_core::records::ProductWithCategory;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use serde_json::json;


pub fn products_query() -> Q {
    Q::from("products").select("*,categories(name,counts_as_item)").null("deleted_at").order("code")
}

fn matches(p: &ProductWithCategory, q: &str, category: &str, inactive: bool) -> bool {
    let p_ = &p.product;
    if !inactive && !p_.is_active {
        return false;
    }
    if !category.is_empty() && p_.category_id.as_deref() != Some(category) {
        return false;
    }
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    [Some(p_.name.as_str()), Some(p_.code.as_str()), p_.source.as_deref()]
        .into_iter()
        .flatten()
        .any(|s| s.to_lowercase().contains(&q))
}

#[component]
pub fn Products() -> impl IntoView {
    let admin = me().role.is_admin();
    let products = live::<ProductWithCategory>("products", products_query());
    let categories = use_categories();
    let sel = Selection::new("products");
    crate::list_position::remember("products", move || products.data.with(Option::is_some));
    let query = use_query_map();
    let navigate = StoredValue::new_local(use_navigate());

    // Filters live in the URL, so Back returns to the same view.
    let param = move |k: &str| query.with(|m| m.get(k).unwrap_or_default());
    let set_param = move |k: &'static str, v: String| {
        let mut m = query.get_untracked();
        if v.is_empty() {
            m.remove(k);
        } else {
            m.replace(k, v);
        }
        let qs = m.to_query_string();
        navigate.with_value(|nav| {
            nav(&format!("/products{qs}"), NavigateOptions { replace: true, scroll: false, ..Default::default() })
        });
    };
    let deleted = RwSignal::new(false);

    let filtered = Memo::new(move |_| {
        let (q, cat, inactive) = (param("q"), param("category"), param("inactive") == "1");
        products.data.with(|rows| {
            rows.as_ref()
                .map(|rows| rows.iter().filter(|p| matches(p, q.trim(), &cat, inactive)).cloned().collect::<Vec<_>>())
        })
    });
    let total = move || filtered.with(|f| f.as_ref().map_or(0, Vec::len));
    let subtitle = move || {
        let n = total();
        let inactive = if param("inactive") == "1" { " including inactive" } else { "" };
        format!("{n} product{}{inactive}", if n == 1 { "" } else { "s" })
    };

    let remove = move |id: String, label: String| {
        if !confirm(&format!(
            "Move {label} to the Bin? It will stay in the Bin for 30 days and can be restored anytime."
        )) {
            return;
        }
        spawn_local(async move {
            let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
            match Q::from("products").eq("id", &id).update(&json!({ "deleted_at": now }), None).await {
                Ok(_) => {
                    products.patch(|rows| rows.retain(|p| p.product.id != id));
                    deleted.set(true);
                    db::cache_drop(&["bin", "catalog"]).await;
                }
                Err(e) => crate::kit::alert(&e.to_string()),
            }
        });
    };

    view! {
        <PageHeader title="Product Master" subtitle=Signal::derive(subtitle)>
            {if admin {
                view! {
                    <Menu
                        label="Excel"
                        items=vec![
                            MenuItem { href: "/admin/products/add", title: "Bulk add", hint: "Add many products from a sheet" },
                            MenuItem { href: "/admin/products/edit", title: "Bulk edit", hint: "Change prices and details in Excel" },
                            MenuItem { href: "/admin/products/export", title: "Download list", hint: "All products as a CSV file" },
                        ]
                    />
                    <A href="/products/history" attr:class="btn-secondary">"History"</A>
                    <A href="/products/new" attr:class="btn-primary">"New product"</A>
                }
                    .into_any()
            } else {
                view! { <A href="/admin/products/export" attr:class="btn-secondary">"Download list"</A> }.into_any()
            }}
            <ListMenu sel=sel />
        </PageHeader>

        <Show when=move || deleted.get()>
            <div class="mb-4 flex items-center justify-between rounded-lg border border-line bg-paper px-4 py-3 text-sm text-(--color-ink) shadow-xs">
                <div class="flex items-center gap-2">
                    <span class="inline-block h-2 w-2 rounded-full bg-emerald-600"></span>
                    <span>"Product moved to Bin. It will remain in the Bin for 30 days and can be restored anytime."</span>
                </div>
                <A href="/bin?tab=products" attr:class="font-semibold text-(--color-brand) underline hover:text-(--color-brand-dark)">"Open Bin →"</A>
            </div>
        </Show>

        <div class="card mb-4 flex flex-wrap items-end gap-3 p-3">
            <div class="min-w-55 flex-1">
                <label class="label" for="q">"Search"</label>
                <input
                    id="q"
                    class="input mt-1"
                    placeholder="Name, code or source"
                    prop:value=move || param("q")
                    on:input=move |e| {
                        set_param("q", event_target_value(&e));
                    }
                />
            </div>
            <div class="min-w-45">
                <label class="label" for="category">"Category"</label>
                <select
                    id="category"
                    class="select mt-1"
                    prop:value=move || param("category")
                    on:change=move |e| set_param("category", event_target_value(&e))
                >
                    <option value="">"All categories"</option>
                    {move || {
                        categories
                            .data
                            .get()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|c| {
                                let selected = param("category") == c.id;
                                view! { <option value=c.id selected=selected>{c.name}</option> }
                            })
                            .collect_view()
                    }}
                </select>
            </div>
            <label class="flex items-center gap-2 pb-1.5 text-sm">
                <input
                    type="checkbox"
                    prop:checked=move || param("inactive") == "1"
                    on:change=move |e| set_param("inactive", if event_target_checked(&e) { "1".into() } else { String::new() })
                />
                "Show inactive"
            </label>
        </div>

        <SelectionCount sel=sel />
        <crate::kit::ErrorBox message=Signal::derive(move || {
            products.error.get().filter(|_| products.data.with(Option::is_none)).map(|e| e.to_string())
        }) />

        {move || match filtered.get() {
            None => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
            Some(rows) if rows.is_empty() => {
                let searching = !param("q").is_empty() || !param("category").is_empty();
                let hint = if admin {
                    "Bulk add them from an Excel sheet, or add one at a time with New product."
                } else {
                    "An admin needs to add products before hampers can be built."
                };
                view! {
                    <EmptyState
                        title=if searching { "No products match that search" } else { "No products yet" }
                        hint=hint
                        action=admin.then_some(("/admin/products/add", "Bulk add from Excel"))
                    />
                }
                    .into_any()
            }
            Some(rows) => {
                view! {
                    // row-gutter (app.css): room left of the table for each row's tick box and bookmark.
                    <div class="list-scroll row-gutter">
                        // No outline: just the rows' own lines, with the gutter's buttons outside them.
                        <div class="rounded-lg bg-white">
                        <table class="w-full border-collapse text-sm">
                            <thead>
                                <tr>
                                    <th class="th num w-12">"S.No."</th>
                                    <th class="th w-12 text-center">"Photo"</th>
                                    <th class="th">"Code"</th>
                                    <th class="th">"Product"</th>
                                    <th class="th">"Category"</th>
                                    <th class="th">"Source"</th>
                                    <th class="th num">"Cost"</th>
                                    <th class="th num">"Markup"</th>
                                    <th class="th num">"Selling price"</th>
                                    <th class="th"></th>
                                </tr>
                            </thead>
                            <tbody>
                                // ponytail: every row at once (fine at ~500 products); bring back
                                // paging or a virtual list if typing in Search starts to lag.
                                {rows
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, p)| view! { <Row p=p n=i + 1 admin=admin sel=sel on_delete=remove /> })
                                    .collect_view()}
                            </tbody>
                        </table>
                        </div>
                    </div>
                }
                    .into_any()
            }
        }}
    }
}

#[component]
fn Row(
    p: ProductWithCategory,
    n: usize,
    admin: bool,
    sel: Selection,
    on_delete: impl Fn(String, String) + Copy + 'static,
) -> impl IntoView {
    let ProductWithCategory { product, categories } = p;
    let (id, code, name) = (product.id.clone(), product.code.clone(), product.name.clone());
    let label = format!("{code} ({name})");
    let photo = match product.image_url.clone() {
        Some(url) => view! {
            <ImagePreview
                src=url
                alt=name.clone()
                class="mx-auto h-9 w-9 rounded-md border border-slate-200 bg-slate-100 shadow-2xs transition-all hover:ring-2 hover:ring-(--color-brand)"
            />
        }
            .into_any(),
        None => view! {
            <A
                href=costing_href(&code)
                attr:title=format!("Add photos for {name}")
                attr:class="mx-auto flex h-9 w-9 items-center justify-center rounded-md border border-dashed border-slate-300 bg-slate-50 text-[10px] font-semibold text-slate-400 transition-colors hover:border-(--color-brand) hover:text-(--color-brand)"
            >
                "+ Pic"
            </A>
        }
            .into_any(),
    };
    let row_class = move || format!("{} {}", ROW_TINT, if product.is_active { "" } else { "opacity-55" });
    let markup = product.markup_pct.map_or_else(|| "—".to_string(), |m| format!("{}%", m.normalize()));
    let (del_id, del_label) = (id.clone(), label.clone());
    let (view_href, edit_href) = (product_href(&code), costing_href(&code));
    let (edit_href2, view_href2) = (edit_href.clone(), view_href.clone());
    let (code_cell, code_label) = (code.clone(), code);
    let category_name = categories.map_or_else(|| "—".to_string(), |c| c.name);
    let source = product.source.clone().unwrap_or_else(|| "—".into());
    let inactive = !product.is_active;
    let (cost, sp) = (format_money(Some(product.cost_price)), format_money(Some(product.default_sp)));
    view! {
        <tr class=row_class>
            <RowNumber sel=sel n=n id=id label=code_label />
            <td class="td w-12 px-2 py-1.5 text-center">{photo}</td>
            <td class="td whitespace-nowrap font-mono">
                <A href=view_href attr:class="font-semibold text-(--color-ink) hover:text-(--color-brand) hover:underline">{code_cell}</A>
            </td>
            <td class="td">
                <A href=view_href2 attr:class="font-medium text-(--color-ink) hover:underline">{name}</A>
                {inactive.then(|| view! { <span class="badge ml-2">"Inactive"</span> })}
            </td>
            <td class="td text-(--color-muted)">{category_name}</td>
            <td class="td text-(--color-muted)">{source}</td>
            <td class="td num">{cost}</td>
            <td class="td num">{markup}</td>
            <td class="td num">{sp}</td>
            <td class="td num">
                <div class="flex items-center justify-end gap-2.5">
                    {if admin {
                        view! {
                            <A href=edit_href attr:class="font-medium text-(--color-ink) hover:text-(--color-brand) hover:underline">"Edit"</A>
                            <button
                                type="button"
                                class="font-medium text-red-700 hover:underline"
                                on:click=move |_| on_delete(del_id.clone(), del_label.clone())
                            >
                                "Delete"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! { <A href=edit_href2 attr:class="font-medium text-(--color-brand) hover:underline">"Costing"</A> }
                            .into_any()
                    }}
                </div>
            </td>
        </tr>
    }
}
