//! Quotes and proforma invoices: the list, the builder and the printed
//! document (ports of `(app)/quotes/*`).

mod builder;
mod print;

use crate::components::{EmptyState, PageHeader};
use crate::data::use_settings;
use crate::db::{Q, live};
use crate::kit::dstr;
pub use builder::{NewQuote, QuotePage};
use lattice_core::Decimal;
use lattice_core::lenient;
use lattice_core::money::format_money;
use lattice_core::records::{DocType, QuoteSummary};
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
pub use print::PrintQuote;
use serde::{Deserialize, Serialize};

pub fn quote_href(doc_no: &str) -> String {
    format!("/quotes/{}", String::from(js_sys::encode_uri_component(doc_no)))
}

pub fn doc_label(t: DocType) -> &'static str {
    match t {
        DocType::Quotation => "Quotation",
        DocType::ProformaInvoice => "Proforma Invoice",
    }
}

fn doc_value(t: DocType) -> &'static str {
    match t {
        DocType::Quotation => "quotation",
        DocType::ProformaInvoice => "proforma_invoice",
    }
}

#[component]
pub fn Quotes() -> impl IntoView {
    let quotes = live::<QuoteSummary>("quotes", Q::from("quote_summary").select("*").order_desc("doc_no"));
    let settings = use_settings();
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
            nav(&format!("/quotes{qs}"), NavigateOptions { replace: true, scroll: false, ..Default::default() })
        });
    };
    let filtered_on = move || ["q", "status", "type"].iter().any(|k| !param(k).trim().is_empty());
    // Filtered on the device as you type: no Apply button, no round trip.
    let rows = Memo::new(move |_| {
        let (q, status, kind) = (param("q").trim().to_lowercase(), param("status"), param("type"));
        let has = |v: &Option<String>| v.as_deref().is_some_and(|v| v.to_lowercase().contains(&q));
        quotes.data.with(|all| {
            all.as_ref().map(|all| {
                all.iter()
                    .filter(|s| {
                        let d = &s.quote;
                        (q.is_empty()
                            || d.client_name.to_lowercase().contains(&q)
                            || d.doc_no.to_lowercase().contains(&q)
                            || has(&d.occasion))
                            && (status.is_empty() || d.status == status)
                            && (kind.is_empty() || doc_value(d.doc_type) == kind)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    });
    let subtitle = move || {
        let n = rows.with(|r| r.as_ref().map_or(0, Vec::len));
        format!("{n} document{}", if n == 1 { "" } else { "s" })
    };

    view! {
        <PageHeader title="Quotes" subtitle=Signal::derive(subtitle)>
            <A href="/quotes/new" attr:class="btn-primary">"New quote"</A>
        </PageHeader>
        <div class="card mb-4 flex flex-wrap items-end gap-3 p-3">
            <div class="min-w-[220px] flex-1">
                <label class="label" for="q">"Search"</label>
                <input id="q" placeholder="Client, document number or occasion" class="input mt-1" prop:value=move || param("q") on:input=move |e| set_param("q", event_target_value(&e)) />
            </div>
            <div class="min-w-[150px]">
                <label class="label" for="type">"Type"</label>
                <select id="type" class="select mt-1" on:change=move |e| set_param("type", event_target_value(&e))>
                    {[("", "Any type"), ("quotation", "Quotation"), ("proforma_invoice", "Proforma Invoice")].map(|(v, label)| view! {
                        <option value=v selected=move || param("type") == v>{label}</option>
                    }).collect_view()}
                </select>
            </div>
            <div class="min-w-[150px]">
                <label class="label" for="status">"Status"</label>
                <select id="status" class="select mt-1" on:change=move |e| set_param("status", event_target_value(&e))>
                    <option value="">"Any status"</option>
                    {move || settings.data.get().map(|s| s.quote_statuses.into_iter().map(|st| {
                        let on = st == param("status");
                        let label = st.clone();
                        view! { <option value=st selected=on>{label}</option> }
                    }).collect_view())}
                </select>
            </div>
            <Show when=filtered_on>
                <A href="/quotes" attr:class="btn-secondary">"Clear"</A>
            </Show>
        </div>
        {move || match rows.get() {
            None => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
            Some(list) if list.is_empty() => view! {
                <EmptyState
                    title=if filtered_on() { "No documents match that search" } else { "No quotations yet" }
                    hint="A quotation pulls saved hampers together for one client, then prints as a document."
                    action=Some(("/quotes/new", "New quote"))
                />
            }.into_any(),
            Some(list) => view! {
                <div class="card overflow-x-auto">
                    <table class="w-full min-w-[1050px] border-collapse text-sm">
                        <thead><tr>
                            <th class="th">"Document"</th><th class="th">"Type"</th><th class="th">"Date"</th><th class="th">"Client"</th>
                            <th class="th">"Occasion"</th><th class="th num">"Hampers"</th><th class="th num">"Qty"</th><th class="th num">"Total"</th>
                            <th class="th">"Status"</th><th class="th">"Follow up"</th>
                        </tr></thead>
                        <tbody>
                            {list.into_iter().map(|s| {
                                let d = s.quote;
                                view! {
                                    <tr>
                                        <td class="td whitespace-nowrap font-mono">
                                            <A href=quote_href(&d.doc_no) attr:class="hover:underline">{d.doc_no.clone()}</A>
                                            {d.linked_doc_no.clone().map(|l| view! { <div class="text-xs text-(--color-muted)">{format!("from {l}")}</div> })}
                                        </td>
                                        <td class="td whitespace-nowrap text-(--color-muted)">{doc_label(d.doc_type)}</td>
                                        <td class="td whitespace-nowrap">{d.doc_date.clone().unwrap_or_default()}</td>
                                        <td class="td">{d.client_name.clone()}</td>
                                        <td class="td text-(--color-muted)">{d.occasion.clone().unwrap_or_else(|| "—".into())}</td>
                                        <td class="td num">{s.line_count}</td>
                                        <td class="td num">{dstr(s.total_qty)}</td>
                                        <td class="td num">
                                            {match s.grand_total {
                                                None => view! { <span class="text-(--color-muted)" title="Option based quotation">"by option"</span> }.into_any(),
                                                Some(t) => format_money(Some(t)).into_any(),
                                            }}
                                        </td>
                                        <td class="td"><span class="badge">{d.status.clone()}</span></td>
                                        <td class="td whitespace-nowrap text-(--color-muted)">{d.follow_up_date.clone().unwrap_or_else(|| "—".into())}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            }.into_any(),
        }}
    }
}

/// The slice of a hamper item the contents block needs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentItem {
    pub id: String,
    pub hamper_id: String,
    pub product_name: String,
    pub category_name: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub qty: Decimal,
}

/// Packaging is whatever Settings marks as not counting toward No. of Items.
pub fn packaging_names(categories: &[lattice_core::records::Category]) -> Vec<String> {
    categories.iter().filter(|c| !c.counts_as_item).map(|c| c.name.trim().to_lowercase()).collect()
}

/// What a quote line says the client is getting (port of `hamper-contents.tsx`).
/// Both pick-lists are free text, matched loosely: "hide" hides the contents,
/// "summary"/"count" prints a count, anything else lists them; "absorb" folds
/// packaging into the hamper, anything else lists it and names the treatment.
/// The builder and the printed sheet both render this, so they agree.
#[component]
pub fn HamperContents(
    items: Vec<ContentItem>,
    detail_mode: String,
    packaging_treatment: String,
    packaging: Vec<String>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let mode = detail_mode.to_lowercase();
    let absorb = packaging_treatment.to_lowercase().contains("absorb");
    let visible: Vec<ContentItem> = items
        .into_iter()
        .filter(|i| !absorb || !packaging.contains(&i.category_name.as_deref().unwrap_or("").trim().to_lowercase()))
        .collect();
    let shown = !mode.contains("hide") && !visible.is_empty();
    let summary = mode.contains("summary") || mode.contains("count");
    let count = visible.len();
    view! {
        <div class=class>
            {shown.then(|| if summary {
                view! { <div class="mt-0.5 text-(--color-muted)">{format!("{count} items")}</div> }.into_any()
            } else {
                view! {
                    <ul class="mt-1 space-y-0.5 text-(--color-muted)">
                        {visible.into_iter().map(|i| {
                            let times = if i.qty == Decimal::ONE { String::new() } else { format!(" × {}", dstr(i.qty)) };
                            view! { <li>{i.product_name}{times}</li> }
                        }).collect_view()}
                    </ul>
                }.into_any()
            })}
            {(!absorb && !packaging_treatment.is_empty()).then(|| view! {
                <div class="mt-1 text-xs text-(--color-muted)">{format!("Packaging: {packaging_treatment}")}</div>
            })}
        </div>
    }
}

/// Opens the system print dialog (Save as PDF lives there). macOS's webview
/// ignores `window.print()`, so the native side prints there.
pub fn print_page() {
    leptos::task::spawn_local(async {
        if !crate::tauri::invoke::<_, bool>("print_page", &serde_json::json!({})).await.unwrap_or(false) {
            let _ = window().print();
        }
    });
}
