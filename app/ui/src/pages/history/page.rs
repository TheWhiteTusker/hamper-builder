//! Every product created, edited, binned or restored, newest first. Admins
//! only (port of `products/history/page.tsx`). The filters live in the URL.

use super::{HISTORY_SELECT, HistoryEntry, HistoryList};
use crate::components::{EmptyState, PageHeader};
use crate::data::use_categories;
use crate::db::{Q, live, live_with, me};
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ACTIONS: [(&str, &str); 5] = [
    ("created", "Created"),
    ("edited", "Edited"),
    ("deleted", "Moved to Bin"),
    ("restored", "Restored"),
    ("purged", "Deleted permanently"),
];
const PAGE: usize = 50;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Person {
    id: String,
    full_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Named {
    code: String,
    name: String,
}

#[component]
pub fn ProductHistoryPage() -> impl IntoView {
    if !me().role.is_admin() {
        return view! {
            <PageHeader title="Product history">""</PageHeader>
            <div class="card p-6 text-sm text-(--color-muted)">"Only an admin can see product history."</div>
        }
        .into_any();
    }
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
        navigate.with_value(|nav| {
            nav(
                &format!("/products/history{qs}"),
                NavigateOptions { replace: true, scroll: false, ..Default::default() },
            )
        });
    };
    // The search waits for a pause in typing, as each change asks the server.
    let typed = StoredValue::new(0u32);
    let on_type = move |v: String| {
        let n = typed.get_value() + 1;
        typed.set_value(n);
        set_timeout(
            move || {
                if typed.get_value() == n {
                    set_param("q", v)
                }
            },
            Duration::from_millis(300),
        );
    };

    let people = live::<Person>("history-people", Q::from("profiles").select("id,full_name").order("full_name"));
    // Binned ones too: their history is still here.
    let products = live::<Named>("history-products", Q::from("products").select("code,name").order("code"));
    let subtitle = RwSignal::new(String::new());
    let filters =
        Memo::new(move |_| [param("q").trim().to_string(), param("person"), param("action"), param("product")]);

    view! {
        <PageHeader title="Product history" subtitle=Signal::derive(move || subtitle.get())>
            <A href="/products" attr:class="btn-secondary">"Products"</A>
        </PageHeader>
        <div class="card mb-4 flex flex-wrap items-end gap-3 p-3">
            <div class="min-w-55 flex-1">
                <label class="label" for="q">"Find a product"</label>
                // Suggests products as you type; picking one shows just its history.
                <input id="q" name="q" type="search" list="history-products" autocomplete="off" class="input mt-1"
                    placeholder="Type a code or name, e.g. HO/0001/WL"
                    prop:value=move || param("q") on:input=move |e| on_type(event_target_value(&e)) />
                <datalist id="history-products">
                    {move || products.data.get().unwrap_or_default().into_iter().map(|p| view! { <option value=p.code>{p.name}</option> }).collect_view()}
                </datalist>
            </div>
            <div class="min-w-45">
                <label class="label" for="person">"Changed by"</label>
                <select id="person" class="select mt-1" on:change=move |e| set_param("person", event_target_value(&e))>
                    <option value="">"Anyone"</option>
                    {move || people.data.get().unwrap_or_default().into_iter().map(|p| {
                        let selected = param("person") == p.id;
                        view! { <option value=p.id selected=selected>{p.full_name.unwrap_or_else(|| "Unnamed".into())}</option> }
                    }).collect_view()}
                </select>
            </div>
            <div class="min-w-45">
                <label class="label" for="action">"Change"</label>
                <select id="action" class="select mt-1" on:change=move |e| set_param("action", event_target_value(&e))>
                    <option value="">"Any change"</option>
                    {ACTIONS.iter().map(|(value, label)| view! { <option value=*value selected=move || param("action") == *value>{*label}</option> }).collect_view()}
                </select>
            </div>
            <Show when=move || !param("product").is_empty()>
                <A href="/products/history" attr:class="pb-1.5 text-sm font-medium text-(--color-brand) hover:underline">"All products"</A>
            </Show>
        </div>
        {move || {
            let [q, person, action, product] = filters.get();
            view! { <Results q person action product subtitle /> }
        }}
    }
    .into_any()
}

#[component]
fn Results(q: String, person: String, action: String, product: String, subtitle: RwSignal<String>) -> impl IntoView {
    let categories = use_categories();
    let filtered = !(q.is_empty() && person.is_empty() && action.is_empty() && product.is_empty());
    let one_product = !product.is_empty();
    let shown = RwSignal::new(PAGE);
    let key = format!("history:{q}|{person}|{action}|{product}");
    let history = live_with(key, move || {
        let mut base = Q::from("product_history").select(HISTORY_SELECT).order_desc("changed_at");
        if !q.is_empty() {
            base = base.search(&["product_code", "product_name"], &q);
        }
        for (col, v) in [("changed_by", &person), ("action", &action), ("product_id", &product)] {
            if !v.is_empty() {
                base = base.eq(col, v);
            }
        }
        let limit = shown.get_untracked();
        async move {
            let (rows, count) =
                futures::future::join(base.clone().limit(limit).rows::<HistoryEntry>(), base.count()).await;
            Ok((rows?, count?))
        }
    });
    Effect::new(move |_| {
        if let Some((entries, total)) = history.data.get() {
            subtitle.set(match entries.first() {
                Some(e) if one_product => format!("{} · {}", e.product_code, e.product_name),
                _ => format!("{total} change{}", if total == 1 { "" } else { "s" }),
            });
        }
    });
    move || match history.data.get() {
        None => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
        Some((entries, _)) if entries.is_empty() => view! {
            <EmptyState
                title=if filtered { "No changes match that search" } else { "No changes recorded yet" }
                hint="Try part of the code or name."
                action=None
            />
        }
        .into_any(),
        Some((entries, total)) => {
            let more = usize::try_from(total).unwrap_or(0).saturating_sub(entries.len());
            view! {
                <div class="card list-scroll">
                    <HistoryList entries=entries categories=categories.data.get().unwrap_or_default() show_product=true />
                    {(more > 0).then(|| view! {
                        <div class="border-t border-line p-3 text-center">
                            <button type="button" class="btn-secondary" disabled=move || history.refreshing.get()
                                on:click=move |_| { shown.update(|n| *n += PAGE * 2); history.refetch(); }>
                                {format!("Show more ({more} left)")}
                            </button>
                        </div>
                    })}
                </div>
            }.into_any()
        }
    }
}
