//! The printed quotation and proforma invoice (port of `[docNo]/print/*`):
//! an HTML sheet, printed (or saved as PDF) through the system dialog.

use super::{ContentItem, HamperContents, packaging_names, print_page, quote_href};
use crate::data::{categories_query, load_settings};
use crate::db::{Q, live_with};
use crate::kit::{dstr, js_str};
use futures::join;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::js::{self, dec, f};
use lattice_core::money::{amount_in_words, format_money, format_pct};
use lattice_core::pricing::{
    COMBINED_ORDER, QuoteLineInput, QuoteOpts, QuotePricing, group_by_option, price_option, price_quote,
};
use lattice_core::records::{Category, DocType, QuoteItem, QuoteSummary};
use lattice_core::settings::CompanySettings;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Printable {
    quote: QuoteSummary,
    lines: Vec<QuoteItem>,
    contents: BTreeMap<String, Vec<ContentItem>>,
    packaging: Vec<String>,
    company: CompanySettings,
}

async fn load_printable(doc_no: String) -> Result<Printable, ApiError> {
    let quote = Q::from("quote_summary")
        .select("*")
        .eq("doc_no", &doc_no)
        .one::<QuoteSummary>()
        .await?
        .ok_or_else(|| ApiError::NotFound("No such quotation.".into()))?;
    let (l, c) =
        (Q::from("quote_items").select("*").eq("quote_id", &quote.quote.id).order("line_no"), categories_query());
    let (lines, settings, categories) = join!(l.rows::<QuoteItem>(), load_settings(), c.rows::<Category>());
    let lines = lines?;
    let ids: Vec<&str> = lines.iter().filter_map(|l| l.hamper_id.as_deref()).collect();
    let mut contents: BTreeMap<String, Vec<ContentItem>> = BTreeMap::new();
    if !ids.is_empty() {
        for item in Q::from("hamper_items")
            .select("*")
            .in_list("hamper_id", &ids)
            .order("line_no")
            .rows::<ContentItem>()
            .await?
        {
            contents.entry(item.hamper_id.clone()).or_default().push(item);
        }
    }
    // Packaging-type categories are the ones Settings marks as not counting
    // toward "No. of Items": the box, the filler, the ribbon.
    Ok(Printable { quote, lines, contents, packaging: packaging_names(&categories?), company: settings?.company })
}

fn priced(lines: &[&QuoteItem]) -> Vec<QuoteLineInput> {
    lines
        .iter()
        .map(|l| QuoteLineInput { qty: l.qty, catalogue_price: l.catalogue_price, discount_pct: l.discount_pct })
        .collect()
}

/// `/quotes/LLQT-012/print`.
#[component]
pub fn PrintQuote() -> impl IntoView {
    let params = use_params_map();
    let doc_no = Memo::new(move |_| {
        params.with(|p| {
            p.get("doc").map(|c| js_sys::decode_uri_component(&c).map(String::from).unwrap_or(c)).unwrap_or_default()
        })
    });
    view! {
        {move || {
            let doc = doc_no.get();
            let d = doc.clone();
            let page = live_with(format!("quote-print:{doc}"), move || load_printable(d.clone()));
            let back = quote_href(&doc);
            view! {
                <div class="no-print mb-4 flex flex-wrap items-center gap-2">
                    <A href=back attr:class="btn-secondary">"Back to quote"</A>
                    <button type="button" class="btn-primary" on:click=move |_| print_page()>"Print / PDF"</button>
                    <p class="text-sm text-(--color-muted)">"Print, then choose “Save as PDF” to send this to a client."</p>
                </div>
                {move || match (page.data.get(), page.error.get()) {
                    (None, Some(e)) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                    (None, None) => view! { <div class="card mx-auto h-[1000px] max-w-[820px] animate-pulse bg-white/60"></div> }.into_any(),
                    (Some(p), _) if p.quote.quote.doc_type == DocType::ProformaInvoice => view! { <ProformaSheet p=p /> }.into_any(),
                    (Some(p), _) => view! { <QuotationSheet p=p /> }.into_any(),
                }}
            }
        }}
    }
}

fn contents_of(p: &Printable, l: &QuoteItem, class: &'static str) -> impl IntoView + use<> {
    let items = l.hamper_id.as_ref().and_then(|id| p.contents.get(id)).cloned().unwrap_or_default();
    view! {
        <HamperContents class=class items=items detail_mode=l.detail_mode.clone().unwrap_or_default()
            packaging_treatment=l.packaging_treatment.clone().unwrap_or_default() packaging=p.packaging.clone() />
    }
}

/// Printed lines of a combined order, or of one option.
fn lines_table(p: &Printable, lines: &[&QuoteItem]) -> impl IntoView + use<> {
    let rows = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            view! {
                <tr class="avoid-break align-top">
                    <td class="td text-(--color-muted)">{i + 1}</td>
                    <td class="td">
                        <div class="font-medium">{l.hamper_name.clone()}</div>
                        {contents_of(p, l, "")}
                    </td>
                    <td class="td num">{dstr(l.qty)}</td>
                    <td class="td num">
                        {format_money(Some(l.final_rate))}
                        {(l.discount_pct > Decimal::ZERO).then(|| view! {
                            <div class="text-xs text-(--color-muted) line-through">{format_money(Some(l.catalogue_price))}</div>
                        })}
                    </td>
                    <td class="td num font-medium">{format_money(Some(l.amount))}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <table class="mt-3 w-full border-collapse">
            <thead><tr>
                <th class="th w-8">"#"</th><th class="th">"Item"</th><th class="th num w-16">"Qty"</th>
                <th class="th num w-24">"Rate"</th><th class="th num w-24">"Amount"</th>
            </tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
}

fn total_row(label: String, value: String, strong: bool) -> impl IntoView {
    view! {
        <div class="flex items-baseline justify-between gap-4">
            <dt class=if strong { "font-semibold" } else { "text-(--color-muted)" }>{label}</dt>
            <dd class=if strong { "tabular-nums text-base font-semibold" } else { "tabular-nums" }>{value}</dd>
        </div>
    }
}

/// The quotation as printed: letterhead, client, lines, totals and terms.
#[component]
fn QuotationSheet(p: Printable) -> impl IntoView {
    let s = &p.quote;
    let q = &s.quote;
    let c = &p.company;
    let all: Vec<&QuoteItem> = p.lines.iter().collect();
    let combined = q.quote_structure == COMBINED_ORDER;
    let title = if q.doc_type == DocType::ProformaInvoice { "Proforma Invoice" } else { "Quotation" };
    let gst_label = format!("GST @ {}", format_pct(Some(q.gst_rate), 0));
    let join = |parts: &[&str]| parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join("  ·  ");
    let contact = join(&[&c.phone, &c.email, &c.website]);
    let client_contact = join(&[q.phone.as_deref().unwrap_or(""), q.email.as_deref().unwrap_or("")]);

    let body = if combined {
        let t = price_quote(
            &priced(&all),
            &QuoteOpts {
                quote_structure: q.quote_structure.clone(),
                order_discount: q.order_discount,
                adj1: q.adj1,
                adj2: q.adj2,
                gst_rate: q.gst_rate,
            },
        );
        let money = |v: Option<Decimal>| format_money(v);
        view! {
            <div class="mt-5">{lines_table(&p, &all)}</div>
            <section class="mt-5 flex justify-end">
                <dl class="w-full max-w-[320px] space-y-1">
                    {total_row("Subtotal".into(), money(Some(t.subtotal)), false)}
                    {(q.order_discount > Decimal::ZERO).then(|| total_row("Order discount".into(), format!("− {}", money(Some(q.order_discount))), false))}
                    {(q.adj1 > Decimal::ZERO).then(|| total_row("Packaging / freight".into(), money(Some(q.adj1)), false))}
                    {(q.adj2 > Decimal::ZERO).then(|| total_row("Other charges".into(), money(Some(q.adj2)), false))}
                    {total_row("Taxable value".into(), money(t.taxable_value), false)}
                    {total_row(gst_label.clone(), money(t.gst_amount), false)}
                    <div class="mt-1 border-t border-(--color-line) pt-1">{total_row("Grand total".into(), money(t.grand_total), true)}</div>
                </dl>
            </section>
        }
        .into_any()
    } else {
        let groups = group_by_option(&p.lines, |l| l.option_label.as_deref());
        view! {
            <p class="mt-5 rounded-md bg-(--color-brand-soft) px-3 py-2 text-(--color-brand-dark)">
                "Each option below is a complete order with its own total. Please let us know which option you would like to proceed with."
            </p>
            {groups.into_iter().map(|g| {
                let t: QuotePricing = price_option(&priced(&g.lines), q.gst_rate);
                view! {
                    <section class="mt-6">
                        <h3 class="text-sm font-semibold uppercase tracking-wide text-(--color-brand-dark)">{g.label.clone()}</h3>
                        {lines_table(&p, &g.lines)}
                        <dl class="avoid-break ml-auto mt-2 w-full max-w-[320px] space-y-1">
                            {total_row("Subtotal".into(), format_money(Some(t.subtotal)), false)}
                            {total_row(gst_label.clone(), format_money(t.gst_amount), false)}
                            <div class="mt-1 border-t border-(--color-line) pt-1">{total_row(format!("{} total", g.label), format_money(t.grand_total), true)}</div>
                        </dl>
                    </section>
                }
            }).collect_view()}
        }
        .into_any()
    };

    view! {
        <article class="print-sheet card mx-auto max-w-[820px] p-10 text-[13px] leading-relaxed">
            <header class="flex flex-wrap items-start justify-between gap-6 border-b border-(--color-line) pb-5">
                <div>
                    <h1 class="text-lg font-semibold tracking-tight">{c.name.clone()}</h1>
                    {(!c.address.is_empty()).then(|| view! { <p class="mt-1 whitespace-pre-line text-(--color-muted)">{c.address.clone()}</p> })}
                    <p class="mt-1 text-(--color-muted)">{contact}</p>
                    {(!c.gstin.is_empty()).then(|| view! { <p class="mt-1 text-(--color-muted)">{format!("GSTIN: {}", c.gstin)}</p> })}
                </div>
                <div class="text-right">
                    <h2 class="text-base font-semibold uppercase tracking-wide">{title}</h2>
                    <p class="mt-1 font-mono text-sm">{q.doc_no.clone()}</p>
                    <p class="text-(--color-muted)">{q.doc_date.clone().unwrap_or_default()}</p>
                    {q.validity.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="text-(--color-muted)">{format!("Valid for {v}")}</p> })}
                    {q.linked_doc_no.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="text-(--color-muted)">{format!("Against {v}")}</p> })}
                </div>
            </header>
            <section class="mt-5 flex flex-wrap justify-between gap-6">
                <div>
                    <div class="label">"Prepared for"</div>
                    <p class="mt-1 font-medium">{q.client_name.clone()}</p>
                    {q.contact_person.clone().filter(|v| !v.is_empty()).map(|v| view! { <p>{v}</p> })}
                    {q.billing_address.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="whitespace-pre-line text-(--color-muted)">{v}</p> })}
                    <p class="text-(--color-muted)">{client_contact}</p>
                    {q.gstin.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="text-(--color-muted)">{format!("GSTIN: {v}")}</p> })}
                </div>
                {q.occasion.clone().filter(|v| !v.is_empty()).map(|v| view! {
                    <div class="text-right"><div class="label">"Occasion"</div><p class="mt-1">{v}</p></div>
                })}
            </section>
            {body}
            {q.terms.clone().filter(|v| !v.is_empty()).map(|v| view! {
                <section class="avoid-break mt-8 border-t border-(--color-line) pt-4">
                    <div class="label">"Terms"</div>
                    <p class="mt-1 whitespace-pre-line text-(--color-muted)">{v}</p>
                </section>
            })}
            <footer class="mt-8 text-center text-xs text-(--color-muted)">
                {if c.website.is_empty() { c.name.clone() } else { format!("{}  ·  {}", c.name, c.website) }}
            </footer>
        </article>
    }
}

/// Blank rows under the items, so a short invoice still reads as the ruled sheet.
const MIN_ROWS: usize = 8;
const LINE: &str = "border-neutral-400";

fn total_line(label: String, value: String, strong: bool) -> impl IntoView {
    view! {
        <div class=format!("flex border-b {LINE} {}", if strong { "bg-neutral-300 text-lg font-bold" } else { "text-sm" })>
            <div class=format!("flex-1 border-r {LINE} px-2 py-1 text-right uppercase")>{label}</div>
            <div class="w-32 px-2 py-1 text-right tabular-nums">{value}</div>
        </div>
    }
}

/// The proforma in the ruled Palm Length LLP format: logo and title, seller /
/// bill-to, the items, the GST split, amount in words, bank details and terms.
#[component]
fn ProformaSheet(p: Printable) -> impl IntoView {
    let q = &p.quote.quote;
    let c = &p.company;
    let all: Vec<&QuoteItem> = p.lines.iter().collect();
    // An invoice always has a total, even if it came from an option-based quote.
    let t = price_quote(
        &priced(&all),
        &QuoteOpts {
            quote_structure: COMBINED_ORDER.into(),
            order_discount: q.order_discount,
            adj1: q.adj1,
            adj2: q.adj2,
            gst_rate: q.gst_rate,
        },
    );
    let (taxable, gst, grand) =
        (t.taxable_value.unwrap_or_default(), t.gst_amount.unwrap_or_default(), t.grand_total.unwrap_or_default());
    let date = q.doc_date.clone().unwrap_or_default().split('-').rev().collect::<Vec<_>>().join("/");
    // A GSTIN starts with the state code: same state splits into CGST + SGST,
    // another state is IGST. No client GSTIN counts as the same state.
    let state = |g: &str| g.chars().take(2).collect::<String>();
    let inter_state =
        q.gstin.as_deref().is_some_and(|g| !g.is_empty() && !c.gstin.is_empty() && state(g) != state(&c.gstin));
    let pct = |rate: f64| format!("{}%", js_str(js::round2(rate * 100.0)));
    let rate = f(q.gst_rate);
    let adjusted = q.order_discount > Decimal::ZERO || q.adj1 > Decimal::ZERO || q.adj2 > Decimal::ZERO;
    let money = |v: Decimal| format_money(Some(v));
    let cell = format!("border-b border-r {LINE} px-2 py-1 last:border-r-0");

    let rows = all
        .iter()
        .enumerate()
        .map(|(i, l)| {
            view! {
                <tr class="align-top">
                    <td class=format!("{cell} text-center")>{i + 1}</td>
                    <td class=cell.clone()>{l.hamper_name.clone()}{contents_of(&p, l, "text-xs")}</td>
                    <td class=format!("{cell} text-center tabular-nums")>{dstr(l.qty)}</td>
                    <td class=format!("{cell} text-right tabular-nums")>{money(l.final_rate)}</td>
                    <td class=format!("{cell} text-right tabular-nums")>{money(l.amount)}</td>
                </tr>
            }
        })
        .collect_view();
    let blanks = (all.len()..MIN_ROWS)
        .map(|_| view! { <tr>{(0..5).map(|_| view! { <td class=format!("{cell} h-6")></td> }).collect_view()}</tr> })
        .collect_view();
    let bank = [
        ("Name", if c.bank_account_name.is_empty() { c.legal_name.clone() } else { c.bank_account_name.clone() }),
        ("Bank", c.bank_name.clone()),
        ("A/c No", c.bank_account_no.clone()),
        ("IFSC Code", c.bank_ifsc.clone()),
    ];

    view! {
        <article class="print-sheet card mx-auto max-w-[820px] p-8 text-[13px] leading-snug text-neutral-800">
            <div class=format!("border {LINE}")>
                <header class=format!("flex items-end justify-between border-b {LINE}")>
                    <img src="/assets/lattice-lane-logo-black.png" alt="Lattice Lane" class="h-36 w-auto" />
                    <h1 class="px-3 pb-2 text-right text-4xl font-light uppercase leading-tight tracking-wide text-neutral-600">"Proforma"<br />"Invoice"</h1>
                </header>
                <section class=format!("grid grid-cols-2 gap-4 border-b {LINE} px-2 py-2")>
                    <div class="space-y-0.5">
                        <p class="text-base">{if c.legal_name.is_empty() { c.name.clone() } else { c.legal_name.clone() }}</p>
                        {(!c.address.is_empty()).then(|| view! { <p class="whitespace-pre-line">{c.address.clone()}</p> })}
                        {(!c.phone.is_empty()).then(|| view! { <p>{format!("Phone No : {}", c.phone)}</p> })}
                        {(!c.email.is_empty()).then(|| view! { <p>{format!("Email ID : {}", c.email)}</p> })}
                        {(!c.gstin.is_empty()).then(|| view! { <p>{format!("GST No: {}", c.gstin)}</p> })}
                    </div>
                    <div class="space-y-0.5 text-right">
                        <p>{format!("PI NO: {}", q.doc_no)}</p>
                        <p>{format!("DATE: {date}")}</p>
                        {q.linked_doc_no.clone().filter(|v| !v.is_empty()).map(|v| view! { <p>{format!("AGAINST: {v}")}</p> })}
                        <p class="pt-1">"BILL TO,"</p>
                        <p class="font-medium uppercase">{q.client_name.clone()}</p>
                        {q.contact_person.clone().filter(|v| !v.is_empty()).map(|v| view! { <p>{format!("Attn: {v}")}</p> })}
                        {q.billing_address.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="whitespace-pre-line">{v}</p> })}
                        {q.gstin.clone().filter(|v| !v.is_empty()).map(|v| view! { <p>{format!("GST No: {v}")}</p> })}
                    </div>
                </section>
                <table class="w-full table-fixed border-collapse">
                    <thead>
                        <tr class="text-sm uppercase">
                            <th class=format!("{cell} w-16 font-normal")>"Sr No"</th>
                            <th class=format!("{cell} font-normal")>"Description"</th>
                            <th class=format!("{cell} w-20 font-normal")>"Qty"</th>
                            <th class=format!("{cell} w-28 font-normal")>"Rate"</th>
                            <th class=format!("{cell} w-32 font-normal")>"Amount"</th>
                        </tr>
                    </thead>
                    <tbody>{rows}{blanks}</tbody>
                </table>
                {total_line("Sub Total".into(), money(t.subtotal), false)}
                {(q.order_discount > Decimal::ZERO).then(|| total_line("Discount".into(), format!("− {}", money(q.order_discount)), false))}
                {(q.adj1 > Decimal::ZERO).then(|| total_line("Packaging / Freight".into(), money(q.adj1), false))}
                {(q.adj2 > Decimal::ZERO).then(|| total_line("Other Charges".into(), money(q.adj2), false))}
                {adjusted.then(|| total_line("Taxable Value".into(), money(taxable), false))}
                {if inter_state {
                    total_line(format!("IGST@{}", pct(rate)), money(gst), false).into_any()
                } else {
                    let half = money(dec(f(gst) / 2.0));
                    view! {
                        {total_line(format!("CGST@{}", pct(rate / 2.0)), half.clone(), false)}
                        {total_line(format!("SGST@{}", pct(rate / 2.0)), half, false)}
                    }.into_any()
                }}
                {total_line("Grand Total".into(), money(grand), true)}
                <div class=format!("flex border-b {LINE}")>
                    <div class=format!("w-24 shrink-0 border-r {LINE} px-2 py-1 uppercase")>"In Words"</div>
                    <div class="px-2 py-1">{amount_in_words(grand)}</div>
                </div>
                <section class=format!("avoid-break grid grid-cols-[2fr_3fr] border-b {LINE}")>
                    <div class=format!("border-r {LINE}")>
                        <p class=format!("border-b {LINE} px-2 py-1 text-sm uppercase")>"Bank Detail:"</p>
                        {bank.into_iter().map(|(label, value)| view! {
                            <p class=format!("border-b {LINE} px-2 py-1.5 text-sm last:border-b-0")>{format!("{label}: {value}")}</p>
                        }).collect_view()}
                    </div>
                    <div>
                        <p class=format!("border-b {LINE} px-2 py-1 text-right text-sm uppercase")>"Terms & Condition"</p>
                        {q.terms.clone().filter(|v| !v.is_empty()).map(|v| view! { <p class="whitespace-pre-line px-2 py-1 text-right text-[11px] leading-snug">{v}</p> })}
                    </div>
                </section>
                <p class="px-3 py-2 text-right text-base uppercase text-neutral-600">"Pleasure doing business with you."</p>
            </div>
        </article>
    }
}
