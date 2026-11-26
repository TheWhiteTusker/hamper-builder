//! The quote builder (port of `quote-builder.tsx` and `_builder/*`): combined
//! orders or side-by-side options, priced on the device, saved via `save_quote`.

use super::{ContentItem, HamperContents, doc_label, packaging_names, quote_href};
use crate::components::PageHeader;
use crate::data::{use_categories, use_settings};
use crate::db::{self, Live, Q, live_with, me};
use crate::kit::{GstinField, confirm, dstr, json_num as n, pct_str};
use futures::join;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::js::{self, dec};
use lattice_core::lenient;
use lattice_core::money::format_money;
use lattice_core::numbers::parse_num;
use lattice_core::pricing::{
    COMBINED_ORDER, QuoteLineInput, QuoteOpts, QuotePricing, group_by_option, option_key, price_option, price_quote,
    price_quote_line,
};
use lattice_core::records::{Client, DocType, Quote, QuoteItem};
use lattice_core::settings::Settings;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_location, use_navigate, use_params_map, use_query_map};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HamperOption {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub final_catalogue_sp: Option<Decimal>,
    pub items: Vec<ContentItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductOption {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_sp: Decimal,
}

/// Everything the builder offers, loaded up front (port of `quotes/data.ts`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuoteOptions {
    pub hampers: Vec<HamperOption>,
    pub products: Vec<ProductOption>,
    pub clients: Vec<Client>,
}

/// ponytail: every hamper's items in one query. Fine for a few hundred
/// hampers; fetch per picked hamper if it ever gets heavy.
async fn load_quote_options() -> Result<QuoteOptions, ApiError> {
    let (h, i, p, c) = (
        Q::from("hamper_summary").select("id,code,name,final_catalogue_sp").order("code"),
        Q::from("hamper_items").select("id,hamper_id,product_name,category_name,qty").order("line_no"),
        // Products can go on a quote on their own, for clients who want one item.
        Q::from("products").select("id,code,name,default_sp").eq("is_active", "true").null("deleted_at").order("code"),
        Q::from("clients").select("*").order("name"),
    );
    let (hampers, items, products, clients) =
        join!(h.rows::<HamperOption>(), i.rows::<ContentItem>(), p.rows::<ProductOption>(), c.rows::<Client>());
    let mut by_hamper: BTreeMap<String, Vec<ContentItem>> = BTreeMap::new();
    for item in items? {
        by_hamper.entry(item.hamper_id.clone()).or_default().push(item);
    }
    let hampers = hampers?
        .into_iter()
        .map(|h| HamperOption { items: by_hamper.remove(&h.id).unwrap_or_default(), ..h })
        .collect();
    Ok(QuoteOptions { hampers, products: products?, clients: clients? })
}

pub fn use_quote_options() -> Live<QuoteOptions> {
    live_with("quote_options", load_quote_options)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct SavedQuote {
    quote: Quote,
    items: Vec<QuoteItem>,
}

async fn load_saved_quote(doc_no: String) -> Result<SavedQuote, ApiError> {
    let quote = Q::from("quotes")
        .select("*")
        .eq("doc_no", &doc_no)
        .one::<Quote>()
        .await?
        .ok_or_else(|| ApiError::NotFound("No such quotation.".into()))?;
    let items = Q::from("quote_items").select("*").eq("quote_id", &quote.id).order("line_no").rows().await?;
    Ok(SavedQuote { quote, items })
}

static NEXT_KEY: AtomicU32 = AtomicU32::new(1);

fn next_key() -> u32 {
    NEXT_KEY.fetch_add(1, Ordering::Relaxed)
}

/// A quote line as typed: numbers stay strings until saved.
#[derive(Clone, Debug, Default, PartialEq)]
struct Line {
    key: u32,
    option_label: String,
    hamper_id: Option<String>,
    hamper_code: String,
    hamper_name: String,
    qty: String,
    catalogue_price: String,
    discount_pct: String,
    detail_mode: String,
    packaging_treatment: String,
}

impl Line {
    fn from_item(i: &QuoteItem) -> Self {
        Self {
            key: next_key(),
            option_label: i.option_label.clone().unwrap_or_default(),
            hamper_id: i.hamper_id.clone(),
            hamper_code: i.hamper_code.clone().unwrap_or_default(),
            hamper_name: i.hamper_name.clone().unwrap_or_default(),
            qty: dstr(i.qty),
            catalogue_price: dstr(i.catalogue_price),
            discount_pct: pct_str(i.discount_pct),
            detail_mode: i.detail_mode.clone().unwrap_or_default(),
            packaging_treatment: i.packaging_treatment.clone().unwrap_or_default(),
        }
    }

    fn from_hamper(h: &HamperOption, option: &str, s: &Settings) -> Self {
        Self {
            key: next_key(),
            option_label: option.into(),
            hamper_id: Some(h.id.clone()),
            hamper_code: h.code.clone(),
            hamper_name: h.name.clone(),
            qty: "1".into(),
            catalogue_price: dstr(h.final_catalogue_sp.unwrap_or_default()),
            discount_pct: "0".into(),
            detail_mode: s.default_detail_mode.clone(),
            packaging_treatment: s.default_packaging_treatment.clone(),
        }
    }

    /// A product quoted on its own rides in the hamper columns with no
    /// hamper_id, so saving, converting and printing need no special case.
    fn from_product(p: &ProductOption, option: &str) -> Self {
        Self {
            key: next_key(),
            option_label: option.into(),
            hamper_code: p.code.clone(),
            hamper_name: p.name.clone(),
            qty: "1".into(),
            catalogue_price: dstr(p.default_sp),
            discount_pct: "0".into(),
            ..Default::default()
        }
    }

    fn priced(&self) -> QuoteLineInput {
        QuoteLineInput {
            qty: parse_num(&self.qty),
            catalogue_price: parse_num(&self.catalogue_price),
            discount_pct: dec(js::num_str(&self.discount_pct) / 100.0),
        }
    }

    fn option(&self) -> String {
        option_key(Some(&self.option_label))
    }
}

/// The first "Option N" not already taken.
fn next_option_label(taken: &[String]) -> String {
    let mut n = taken.len() + 1;
    while taken.contains(&format!("Option {n}")) {
        n += 1;
    }
    format!("Option {n}")
}

/// Everything on the quote besides its lines. Percentages are whole numbers here.
#[derive(Clone, Copy)]
struct Fields {
    doc_type: RwSignal<DocType>,
    doc_date: RwSignal<String>,
    client_name: RwSignal<String>,
    contact_person: RwSignal<String>,
    phone: RwSignal<String>,
    email: RwSignal<String>,
    billing_address: RwSignal<String>,
    gstin: RwSignal<String>,
    occasion: RwSignal<String>,
    structure: RwSignal<String>,
    validity: RwSignal<String>,
    status: RwSignal<String>,
    follow_up: RwSignal<String>,
    notes: RwSignal<String>,
    terms: RwSignal<String>,
    order_discount: RwSignal<String>,
    adj1: RwSignal<String>,
    adj2: RwSignal<String>,
    gst_rate: RwSignal<String>,
}

fn today() -> String {
    // As `new Date().toISOString().slice(0, 10)` did: the UTC date.
    js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default().chars().take(10).collect()
}

impl Fields {
    fn new(q: Option<&Quote>, s: &Settings) -> Self {
        let text = |f: fn(&Quote) -> Option<String>| RwSignal::new(q.and_then(f).unwrap_or_default());
        Self {
            doc_type: RwSignal::new(q.map(|q| q.doc_type).unwrap_or_default()),
            doc_date: RwSignal::new(q.and_then(|q| q.doc_date.clone()).unwrap_or_else(today)),
            client_name: RwSignal::new(q.map(|q| q.client_name.clone()).unwrap_or_default()),
            contact_person: text(|q| q.contact_person.clone()),
            phone: text(|q| q.phone.clone()),
            email: text(|q| q.email.clone()),
            billing_address: text(|q| q.billing_address.clone()),
            gstin: text(|q| q.gstin.clone()),
            occasion: text(|q| q.occasion.clone()),
            structure: RwSignal::new(q.map_or_else(|| COMBINED_ORDER.to_string(), |q| q.quote_structure.clone())),
            validity: RwSignal::new(
                q.map_or_else(|| Some(s.default_validity.clone()), |q| q.validity.clone()).unwrap_or_default(),
            ),
            status: RwSignal::new(q.map_or_else(|| "Draft".to_string(), |q| q.status.clone())),
            follow_up: text(|q| q.follow_up_date.clone()),
            notes: text(|q| q.notes.clone()),
            terms: RwSignal::new(
                q.map_or_else(|| Some(s.quote_terms.clone()), |q| q.terms.clone()).unwrap_or_default(),
            ),
            order_discount: RwSignal::new(q.map_or_else(|| "0".into(), |q| dstr(q.order_discount))),
            adj1: RwSignal::new(q.map_or_else(|| "0".into(), |q| dstr(q.adj1))),
            adj2: RwSignal::new(q.map_or_else(|| "0".into(), |q| dstr(q.adj2))),
            gst_rate: RwSignal::new(pct_str(q.map_or(s.gst_rate, |q| q.gst_rate))),
        }
    }

    fn gst(&self) -> Decimal {
        dec(js::num_str(&self.gst_rate.get()) / 100.0)
    }
}

/// `/quotes/new` (optionally `?hamper=H128`, from a hamper's "Add to a quote").
#[component]
pub fn NewQuote() -> impl IntoView {
    let query = use_query_map();
    let seed = query.with_untracked(|q| q.get("hamper")).filter(|h| !h.is_empty());
    view! {
        <PageHeader title="New quote" subtitle="The document number is assigned when you save">""</PageHeader>
        <Screen saved=None seed=seed />
    }
}

/// `/quotes/LLQT-012`: a saved quotation or proforma, editable by its owner
/// or a manager.
#[component]
pub fn QuotePage() -> impl IntoView {
    let params = use_params_map();
    let location = use_location();
    let doc_no = Memo::new(move |_| {
        params.with(|p| {
            p.get("doc").map(|c| js_sys::decode_uri_component(&c).map(String::from).unwrap_or(c)).unwrap_or_default()
        })
    });
    // Keyed on the whole URL: saving lands back here (`?saved=1`), and the
    // page then starts again from what the server stored.
    let page_key = Memo::new(move |_| (doc_no.get(), location.search.get()));
    let query = use_query_map();
    view! {
        {move || {
            let (doc, _) = page_key.get();
            let d = doc.clone();
            let page = live_with(format!("quote:{doc}"), move || load_saved_quote(d.clone()));
            let first = RwSignal::new(None::<SavedQuote>);
            Effect::new(move |_| {
                if first.with_untracked(Option::is_none)
                    && let Some(p) = page.data.get()
                {
                    first.set(Some(p));
                }
            });
            let converted = query.with_untracked(|q| q.get("converted")).filter(|c| !c.is_empty());
            view! {
                <Show
                    when=move || first.with(Option::is_some)
                    fallback=move || match page.error.get() {
                        Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                        None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
                    }
                >
                    {
                        let converted = converted.clone();
                        move || first.get_untracked().map(|p| {
                            let q = p.quote.clone();
                            let profile = me();
                            // Sales may edit their own quotations; managers and admins any.
                            let mine = q.created_by.as_deref() == Some(profile.id.as_str());
                            let can_edit = profile.role.can_manage() || mine;
                            view! {
                                <PageHeader title=q.client_name.clone() subtitle=format!("{} {}", doc_label(q.doc_type), q.doc_no)>
                                    <span class="badge">{q.status.clone()}</span>
                                </PageHeader>
                                {converted.clone().map(|from| view! {
                                    <p class="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">{format!("Created from quotation {from}.")}</p>
                                })}
                                {(!can_edit).then(|| view! {
                                    <p class="mb-4 rounded-md border border-(--color-line) bg-white px-3 py-2 text-sm text-(--color-muted)">
                                        "This quotation belongs to someone else, so it is read only for you."
                                    </p>
                                })}
                                <Screen saved=Some(p) seed=None />
                            }
                        })
                    }
                </Show>
            }
        }}
    }
}

/// Waits for the pick-lists, then mounts the builder once.
#[component]
fn Screen(saved: Option<SavedQuote>, seed: Option<String>) -> impl IntoView {
    let options = use_quote_options();
    let settings = use_settings();
    let categories = use_categories();
    let ready = RwSignal::new(None::<(QuoteOptions, Settings)>);
    Effect::new(move |_| {
        if ready.with_untracked(Option::is_some) {
            return;
        }
        if let (Some(o), Some(s)) = (options.data.get(), settings.data.get()) {
            ready.set(Some((o, s)));
        }
    });
    let packaging = Memo::new(move |_| categories.data.with(|c| c.as_deref().map(packaging_names).unwrap_or_default()));
    let saved = StoredValue::new(saved);
    let seed = StoredValue::new(seed);
    view! {
        <Show
            when=move || ready.with(Option::is_some)
            fallback=move || match options.error.get() {
                Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
            }
        >
            {move || ready.get_untracked().map(|(o, s)| view! {
                <Builder saved=saved.get_value() seed=seed.get_value() options=o settings=s packaging=packaging />
            })}
        </Show>
    }
}

#[component]
fn Builder(
    saved: Option<SavedQuote>,
    seed: Option<String>,
    options: QuoteOptions,
    settings: Settings,
    packaging: Memo<Vec<String>>,
) -> impl IntoView {
    let profile = me();
    let can_edit = profile.role.can_manage()
        || saved.as_ref().is_some_and(|s| s.quote.created_by.as_deref() == Some(profile.id.as_str()));
    let can_change_status = profile.role.can_manage();
    let navigate = StoredValue::new_local(use_navigate());
    let go = move |to: String| navigate.with_value(|nav| nav(&to, Default::default()));

    let fields = Fields::new(saved.as_ref().map(|s| &s.quote), &settings);
    let first: Vec<Line> = match &saved {
        Some(s) => s.items.iter().map(Line::from_item).collect(),
        // Arrived from a hamper page via "Add to a quote".
        None => seed
            .and_then(|code| options.hampers.iter().find(|h| h.code == code))
            .map(|h| vec![Line::from_hamper(h, "Option 1", &settings)])
            .unwrap_or_default(),
    };
    let lines = RwSignal::new(first);
    // Options added but still empty: otherwise an option exists only through its lines.
    let empty = RwSignal::new(Vec::<String>::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(None::<&'static str>);
    let opts = StoredValue::new(options);
    let settings = StoredValue::new(settings);
    let saved_quote = StoredValue::new(saved.as_ref().map(|s| s.quote.clone()));
    // Saved options, not unsaved edits: converting reads the stored quotation.
    let saved_options: Vec<String> = saved
        .as_ref()
        .filter(|s| s.quote.quote_structure != COMBINED_ORDER)
        .map(|s| group_by_option(&s.items, |i| i.option_label.as_deref()).into_iter().map(|g| g.label).collect())
        .unwrap_or_default();

    let combined = Memo::new(move |_| fields.structure.get() == COMBINED_ORDER);
    let totals = Memo::new(move |_| {
        let priced: Vec<QuoteLineInput> = lines.with(|ls| ls.iter().map(Line::priced).collect());
        price_quote(
            &priced,
            &QuoteOpts {
                quote_structure: fields.structure.get(),
                order_discount: parse_num(&fields.order_discount.get()),
                adj1: parse_num(&fields.adj1.get()),
                adj2: parse_num(&fields.adj2.get()),
                gst_rate: fields.gst(),
            },
        )
    });
    // Each option as listed: those with lines in first-seen order, then empty ones.
    let groups = Memo::new(move |_| {
        let mut labels: Vec<String> = Vec::new();
        lines.with(|ls| {
            for l in ls {
                let k = l.option();
                if !labels.contains(&k) {
                    labels.push(k);
                }
            }
        });
        empty.with(|e| labels.extend(e.iter().filter(|x| !labels.contains(x)).cloned().collect::<Vec<_>>()));
        if labels.is_empty() { vec!["Option 1".to_string()] } else { labels }
    });
    let option_totals = Memo::new(move |_| {
        let gst = fields.gst();
        groups
            .get()
            .into_iter()
            .map(|g| {
                let priced: Vec<QuoteLineInput> =
                    lines.with(|ls| ls.iter().filter(|l| l.option() == g).map(Line::priced).collect());
                (g, price_option(&priced, gst))
            })
            .collect::<Vec<_>>()
    });

    let add = move |kind: &str, id: String, option: String| {
        let line = opts.with_value(|o| match kind {
            "hamper" => {
                o.hampers.iter().find(|h| h.id == id).map(|h| settings.with_value(|s| Line::from_hamper(h, &option, s)))
            }
            _ => o.products.iter().find(|p| p.id == id).map(|p| Line::from_product(p, &option)),
        });
        if let Some(line) = line {
            lines.update(|ls| ls.push(line));
        }
    };
    // Copies the client onto the document, so a sent quote keeps the address
    // it went out with even if the client record changes later.
    let pick_client = move |id: String| {
        let Some(c) = opts.with_value(|o| o.clients.iter().find(|c| c.id == id).cloned()) else { return };
        fields.client_name.set(c.name);
        fields.contact_person.set(c.contact_person.unwrap_or_default());
        fields.phone.set(c.phone.unwrap_or_default());
        fields.email.set(c.email.unwrap_or_default());
        fields.billing_address.set(c.billing_address.unwrap_or_default());
        fields.gstin.set(c.gstin.unwrap_or_default());
    };
    // Quotations and proformas have different default terms; swap them only
    // while the terms are still the untouched default.
    let change_doc_type = move |next: DocType| {
        let default_for = |t: DocType| {
            settings.with_value(|s| match t {
                DocType::Quotation => s.quote_terms.clone(),
                DocType::ProformaInvoice if s.pi_terms.is_empty() => s.quote_terms.clone(),
                DocType::ProformaInvoice => s.pi_terms.clone(),
            })
        };
        if fields.terms.get_untracked() == default_for(fields.doc_type.get_untracked()) {
            fields.terms.set(default_for(next));
        }
        fields.doc_type.set(next);
    };

    let save = move || {
        error.set(None);
        let text = |s: String| Some(s.trim().to_string()).filter(|s| !s.is_empty());
        let (date, client) =
            (fields.doc_date.get_untracked().trim().to_string(), fields.client_name.get_untracked().trim().to_string());
        let ls = lines.get_untracked();
        let problem = if date.is_empty() {
            Some("Pick the document date.")
        } else if client.is_empty() {
            Some("Client or company name is required")
        } else if ls.is_empty() {
            Some("Add at least one hamper or product to the quotation")
        } else {
            None
        };
        if let Some(p) = problem {
            return error.set(Some(p.into()));
        }
        let structure = fields.structure.get_untracked().trim().to_string();
        let is_combined = structure == COMBINED_ORDER;
        let payload = json!({
            "id": saved_quote.with_value(|q| q.as_ref().map(|q| q.id.clone())),
            "doc_type": fields.doc_type.get_untracked(),
            "doc_date": date,
            "client_name": client,
            "contact_person": text(fields.contact_person.get_untracked()),
            "phone": text(fields.phone.get_untracked()),
            "email": text(fields.email.get_untracked()),
            "billing_address": text(fields.billing_address.get_untracked()),
            "gstin": text(fields.gstin.get_untracked()),
            "occasion": text(fields.occasion.get_untracked()),
            "quote_structure": structure,
            "validity": text(fields.validity.get_untracked()),
            "status": fields.status.get_untracked().trim(),
            "order_discount": n(js::num_str(&fields.order_discount.get_untracked())),
            "adj1": n(js::num_str(&fields.adj1.get_untracked())),
            "adj2": n(js::num_str(&fields.adj2.get_untracked())),
            "gst_rate": n(js::num_str(&fields.gst_rate.get_untracked()) / 100.0),
            "notes": text(fields.notes.get_untracked()),
            "terms": text(fields.terms.get_untracked()),
            "follow_up_date": text(fields.follow_up.get_untracked()),
            "linked_doc_no": saved_quote.with_value(|q| q.as_ref().and_then(|q| q.linked_doc_no.clone())),
            "lines": ls.iter().map(|l| json!({
                // A combined order is one set of confirmed items: no options.
                "option_label": if is_combined { None } else { Some(l.option()) },
                "hamper_id": l.hamper_id,
                "hamper_code": text(l.hamper_code.clone()),
                "hamper_name": text(l.hamper_name.clone()),
                "qty": n(js::num_str(&l.qty)),
                "catalogue_price": n(js::num_str(&l.catalogue_price)),
                "discount_pct": n(js::num_str(&l.discount_pct) / 100.0),
                "detail_mode": text(l.detail_mode.clone()),
                "packaging_treatment": text(l.packaging_treatment.clone()),
            })).collect::<Vec<_>>(),
        });
        let old = saved_quote.with_value(|q| q.as_ref().map(|q| q.doc_no.clone()));
        busy.set(Some("save"));
        spawn_local(async move {
            match db::rpc::<_, String>("save_quote", &json!({ "p": payload })).await {
                Ok(doc) => {
                    let mut stale = vec![
                        "quotes".to_string(),
                        "dashboard".into(),
                        format!("quote:{doc}"),
                        format!("quote-print:{doc}"),
                    ];
                    stale.extend(old.map(|d| format!("quote:{d}")));
                    db::cache_drop(&stale.iter().map(String::as_str).collect::<Vec<_>>()).await;
                    go(format!("{}?saved=1", quote_href(&doc)));
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    view! {
        <div class="space-y-4">
            <Details fields=fields clients=opts.with_value(|o| o.clients.clone()) settings=settings.get_value() can_edit=can_edit can_change_status=can_change_status
                pick_client=pick_client change_doc_type=change_doc_type />

            <section class="card overflow-hidden">
                <div class="flex flex-wrap items-end justify-between gap-3 border-b border-(--color-line) px-4 py-2.5">
                    <div>
                        <h2 class="text-sm font-semibold">"Hampers & products"</h2>
                        <p class="text-xs text-(--color-muted)">
                            {move || if combined.get() {
                                "Combined order: the confirmed items, with one total."
                            } else {
                                "Option based: each option is its own set of items with its own total, for the client to choose from."
                            }}
                        </p>
                    </div>
                    <div class="flex flex-wrap items-end gap-2">
                        <div>
                            <label class="label" for="structure">"Structure"</label>
                            <select id="structure" class="select mt-1" disabled=!can_edit on:change=move |e| fields.structure.set(event_target_value(&e))>
                                {move || {
                                    let current = fields.structure.get();
                                    let mut list = settings.with_value(|s| s.quote_structures.clone());
                                    if !list.contains(&current) {
                                        list.push(current.clone());
                                    }
                                    list.into_iter().map(|s| {
                                        let on = s == current;
                                        let label = s.clone();
                                        view! { <option value=s selected=on>{label}</option> }
                                    }).collect_view()
                                }}
                            </select>
                        </div>
                        <Show when=move || can_edit && combined.get()>
                            <Pickers option=Signal::stored("Option 1".to_string()) lines=lines opts=opts add=add />
                        </Show>
                    </div>
                </div>
                <Show
                    when=move || combined.get()
                    fallback=move || view! {
                        // Keyed by position: renaming an option must not remount its section.
                        <For each=move || 0..groups.with(Vec::len) key=|i| *i let:i>
                            <OptionGroup index=i groups=groups option_totals=option_totals lines=lines empty=empty fields=fields
                                opts=opts packaging=packaging settings=settings can_edit=can_edit add=add />
                        </For>
                        {can_edit.then(|| view! {
                            <div class="border-t border-(--color-line) px-4 py-3">
                                <button type="button" class="btn-secondary" on:click=move |_| {
                                    let had_lines = lines.with(|ls| !ls.is_empty()) || empty.with(|e| !e.is_empty());
                                    let taken: Vec<String> = groups.get_untracked().into_iter().chain(empty.get_untracked()).collect();
                                    let next = next_option_label(&taken);
                                    empty.update(|e| {
                                        if !had_lines {
                                            e.push("Option 1".into());
                                        }
                                        e.push(next);
                                    });
                                }>"+ Add option"</button>
                            </div>
                        })}
                    }
                >
                    <LineTable lines=lines option=None opts=opts packaging=packaging settings=settings can_edit=can_edit />
                </Show>
            </section>

            <Totals fields=fields totals=totals option_totals=option_totals combined=combined can_edit=can_edit />

            {move || error.get().map(|e| view! { <p role="alert" class="text-sm text-red-700">{e}</p> })}

            {can_edit.then(|| view! {
                <div class="flex flex-wrap items-center gap-2">
                    <button type="button" class="btn-primary" disabled=move || busy.get().is_some() on:click=move |_| save()>
                        {move || match busy.get() {
                            Some("save") => "Saving…",
                            _ if saved_quote.with_value(Option::is_some) => "Save changes",
                            _ => "Create quote",
                        }}
                    </button>
                    <A href="/quotes" attr:class="btn-secondary">"Cancel"</A>
                </div>
            })}
        </div>
        {saved_quote.get_value().map(|q| view! { <Actions quote=q can_edit=can_edit options=saved_options busy=busy error=error /> })}
    }
}

#[component]
fn Field(
    label: &'static str,
    id: &'static str,
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=class>
            <label class="label" for=id>{label}</label>
            {children()}
        </div>
    }
}

#[component]
fn TextField(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    disabled: bool,
    #[prop(optional)] kind: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] mode: &'static str,
) -> impl IntoView {
    let kind = if kind.is_empty() { "text" } else { kind };
    view! {
        <Field label=label id=id>
            <input id=id type=kind inputmode=(!mode.is_empty()).then_some(mode) class=format!("input mt-1 {class}") disabled=disabled
                prop:value=value on:input=move |e| value.set(event_target_value(&e)) />
        </Field>
    }
}

/// Document type, date, status and the client's details.
#[component]
fn Details(
    fields: Fields,
    clients: Vec<Client>,
    settings: Settings,
    can_edit: bool,
    can_change_status: bool,
    pick_client: impl Fn(String) + Copy + Send + Sync + 'static,
    change_doc_type: impl Fn(DocType) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let off = !can_edit;
    let statuses = settings.quote_statuses.clone();
    let validity = settings.validity_options.clone();
    view! {
        <section class="card p-4">
            {can_edit.then(|| {
                let placeholder = if clients.is_empty() { "No saved clients yet" } else { "Fill details from a saved client…" };
                view! {
                    <div class="mb-4 flex flex-wrap items-end gap-2 border-b border-(--color-line) pb-4">
                        <Field label="Saved client" id="savedClient" class="min-w-[280px]">
                            <select id="savedClient" class="select mt-1" prop:value="" on:change=move |e| {
                                pick_client(event_target_value(&e));
                                if let Some(el) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlSelectElement>(t).ok()) {
                                    el.set_value("");
                                }
                            }>
                                <option value="">{placeholder}</option>
                                {clients.into_iter().map(|c| {
                                    let label = match &c.gstin { Some(g) if !g.is_empty() => format!("{} — {g}", c.name), _ => c.name.clone() };
                                    view! { <option value=c.id>{label}</option> }
                                }).collect_view()}
                            </select>
                        </Field>
                        <A href="/clients" attr:class="btn-secondary">"Manage clients"</A>
                    </div>
                }
            })}
            <div class="grid gap-4 md:grid-cols-3">
                <Field label="Document type" id="docType">
                    <select id="docType" class="select mt-1" disabled=off on:change=move |e| change_doc_type(if event_target_value(&e) == "proforma_invoice" { DocType::ProformaInvoice } else { DocType::Quotation })>
                        <option value="quotation" selected=move || fields.doc_type.get() == DocType::Quotation>"Quotation"</option>
                        <option value="proforma_invoice" selected=move || fields.doc_type.get() == DocType::ProformaInvoice>"Proforma Invoice"</option>
                    </select>
                </Field>
                <TextField id="docDate" label="Date" kind="date" value=fields.doc_date disabled=off />
                <Field label="Status" id="status">
                    <select id="status" class="select mt-1" disabled={off || !can_change_status} on:change=move |e| fields.status.set(event_target_value(&e))>
                        {move || {
                            let current = fields.status.get();
                            let mut list = statuses.clone();
                            if !list.contains(&current) {
                                list.push(current.clone());
                            }
                            list.into_iter().map(|s| {
                                let on = s == current;
                                let label = s.clone();
                                view! { <option value=s selected=on>{label}</option> }
                            }).collect_view()
                        }}
                    </select>
                    {(!can_change_status).then(|| view! { <p class="mt-1 text-xs text-(--color-muted)">"Only a manager can change status."</p> })}
                </Field>
                <TextField id="clientName" label="Client / company" value=fields.client_name disabled=off />
                <TextField id="contactPerson" label="Contact person" value=fields.contact_person disabled=off />
                <TextField id="occasion" label="Occasion / project" value=fields.occasion disabled=off />
                <TextField id="email" label="Email" kind="email" value=fields.email disabled=off />
                <TextField id="phone" label="Phone" value=fields.phone disabled=off />
                <GstinField id="gstin" label="GSTIN" value=fields.gstin can_edit=can_edit on_found=move |name: String, address: String| {
                    if !name.is_empty() {
                        fields.client_name.set(name);
                    }
                    if !address.is_empty() {
                        fields.billing_address.set(address);
                    }
                } />
                <Field label="Billing address" id="billingAddress" class="md:col-span-2">
                    <textarea id="billingAddress" rows="2" class="input mt-1" disabled=off prop:value=fields.billing_address
                        on:input=move |e| fields.billing_address.set(event_target_value(&e))></textarea>
                </Field>
                <div class="grid gap-4">
                    <Field label="Validity" id="validity">
                        <input id="validity" list="validity-options" class="input mt-1" disabled=off prop:value=fields.validity
                            on:input=move |e| fields.validity.set(event_target_value(&e)) />
                        <datalist id="validity-options">
                            {validity.into_iter().map(|v| view! { <option value=v></option> }).collect_view()}
                        </datalist>
                    </Field>
                    <TextField id="followUp" label="Follow up on" kind="date" value=fields.follow_up disabled=off />
                </div>
            </div>
        </section>
    }
}

/// "Add hamper" and "Add product" selects for one option; each resets after a pick.
#[component]
fn Pickers(
    #[prop(into)] option: Signal<String>,
    lines: RwSignal<Vec<Line>>,
    opts: StoredValue<QuoteOptions>,
    add: impl Fn(&str, String, String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let added = Memo::new(move |_| {
        let o = option.get();
        lines.with(|ls| ls.iter().filter(|l| l.option() == o).map(|l| l.hamper_code.clone()).collect::<HashSet<_>>())
    });
    let picker = move |kind: &'static str,
                       label: &'static str,
                       placeholder: &'static str,
                       items: Vec<(String, String, String)>| {
        let id = move || format!("{kind}-{}", option.get().replace(|c: char| !c.is_alphanumeric() && c != '_', "-"));
        view! {
            <div>
                <label class="label" for=id>{label}</label>
                <select id=id class="select mt-1 min-w-[220px]" on:change=move |e| {
                    let v = event_target_value(&e);
                    if let Some(el) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlSelectElement>(t).ok()) {
                        el.set_value("");
                    }
                    if !v.is_empty() {
                        add(kind, v, option.get_untracked());
                    }
                }>
                    <option value="" selected=true>{placeholder}</option>
                    {move || added.with(|added| items.iter().map(|(id, code, name)| {
                        let tag = if added.contains(code) { " (already added)" } else { "" };
                        view! { <option value=id.clone()>{format!("{code} — {name}{tag}")}</option> }
                    }).collect_view())}
                </select>
            </div>
        }
    };
    let (hampers, products) = opts.with_value(|o| {
        (
            o.hampers.iter().map(|h| (h.id.clone(), h.code.clone(), h.name.clone())).collect::<Vec<_>>(),
            o.products.iter().map(|p| (p.id.clone(), p.code.clone(), p.name.clone())).collect::<Vec<_>>(),
        )
    });
    view! {
        {picker("hamper", "Add hamper", "Select a hamper…", hampers)}
        {picker("product", "Add product", "Select a product…", products)}
    }
}

/// One option of an option-based quote: its name, pickers, lines and total.
/// The name commits on blur, so typing doesn't regroup lines per keystroke.
#[component]
fn OptionGroup(
    index: usize,
    groups: Memo<Vec<String>>,
    option_totals: Memo<Vec<(String, QuotePricing)>>,
    lines: RwSignal<Vec<Line>>,
    empty: RwSignal<Vec<String>>,
    fields: Fields,
    opts: StoredValue<QuoteOptions>,
    packaging: Memo<Vec<String>>,
    settings: StoredValue<Settings>,
    can_edit: bool,
    add: impl Fn(&str, String, String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let label = Memo::new(move |_| groups.with(|g| g.get(index).cloned().unwrap_or_default()));
    let totals = Memo::new(move |_| option_totals.with(|t| t.get(index).map(|(_, p)| p.clone())));
    let input = NodeRef::<leptos::html::Input>::new();
    let commit = move || {
        let Some(el) = input.get_untracked() else { return };
        let (next, current) = (el.value().trim().to_string(), label.get_untracked());
        if next.is_empty() || next == current {
            return el.set_value(&current);
        }
        let has_lines = lines.with_untracked(|ls| ls.iter().any(|l| l.option() == current));
        // An empty option has no lines to carry the name, so it is kept here.
        empty.update(|e| {
            if let Some(x) = e.iter_mut().find(|x| **x == current) {
                *x = next.clone();
            } else if !has_lines {
                e.push(next.clone());
            }
        });
        lines.update(|ls| ls.iter_mut().filter(|l| l.option() == current).for_each(|l| l.option_label = next.clone()));
    };
    let remove = move || {
        let current = label.get_untracked();
        empty.update(|e| e.retain(|x| *x != current));
        lines.update(|ls| ls.retain(|l| l.option() != current));
    };
    let money = move |pick: fn(&QuotePricing) -> Option<Decimal>| {
        move || totals.with(|t| t.as_ref().and_then(pick)).map_or_else(String::new, |v| format_money(Some(v)))
    };
    view! {
        <div class="border-t border-(--color-line) first:border-t-0">
            <div class="flex flex-wrap items-end justify-between gap-3 bg-(--color-brand-soft) px-4 py-2.5">
                <div class="flex items-end gap-2">
                    <div>
                        <label class="label" for=move || format!("opt-{}", label.get())>"Option name"</label>
                        <input node_ref=input id=move || format!("opt-{}", label.get()) class="input mt-1 max-w-[180px] font-semibold" disabled=!can_edit
                            prop:value=move || label.get() on:blur=move |_| commit()
                            on:keydown=move |e| if e.key() == "Enter" { e.prevent_default(); commit(); } />
                    </div>
                    {can_edit.then(|| view! { <button type="button" class="btn-secondary text-xs text-red-700" on:click=move |_| remove()>"Remove option"</button> })}
                </div>
                {can_edit.then(|| view! {
                    <div class="flex flex-wrap items-end gap-2"><Pickers option=label lines=lines opts=opts add=add /></div>
                })}
            </div>
            <LineTable lines=lines option=Some(label) opts=opts packaging=packaging settings=settings can_edit=can_edit />
            <dl class="ml-auto flex max-w-[520px] flex-wrap justify-end gap-x-6 gap-y-1 px-4 py-2.5 text-sm">
                <Total label=Signal::stored("Subtotal".to_string()) value=Signal::derive(money(|t| Some(t.subtotal))) />
                <Total label=Signal::derive(move || { let g = fields.gst_rate.get(); format!("GST {}%", if g.is_empty() { "0".into() } else { g }) }) value=Signal::derive(money(|t| t.gst_amount)) />
                <Total label=Signal::derive(move || format!("{} total", label.get())) value=Signal::derive(money(|t| t.grand_total)) strong=true />
            </dl>
        </div>
    }
}

#[component]
fn Total(
    #[prop(into)] label: Signal<String>,
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] strong: bool,
) -> impl IntoView {
    view! {
        <div class="flex items-baseline gap-2">
            <dt class="text-(--color-muted)">{label}</dt>
            <dd class=if strong { "tabular-nums text-base font-semibold" } else { "tabular-nums font-medium" }>{value}</dd>
        </div>
    }
}

/// The editable lines of a combined order (`option` None), or of one option.
#[component]
fn LineTable(
    lines: RwSignal<Vec<Line>>,
    option: Option<Memo<String>>,
    opts: StoredValue<QuoteOptions>,
    packaging: Memo<Vec<String>>,
    settings: StoredValue<Settings>,
    can_edit: bool,
) -> impl IntoView {
    let keys = Memo::new(move |_| {
        let want = option.map(|o| o.get());
        lines.with(|ls| {
            ls.iter().filter(|l| want.as_ref().is_none_or(|w| l.option() == *w)).map(|l| l.key).collect::<Vec<_>>()
        })
    });
    view! {
        <Show
            when=move || keys.with(|k| !k.is_empty())
            fallback=|| view! { <p class="px-4 py-6 text-center text-sm text-(--color-muted)">"Nothing added yet. Add a hamper or a single product above."</p> }
        >
            <div class="overflow-x-auto">
                <table class="w-full min-w-[1000px] border-collapse text-sm">
                    <thead><tr>
                        <th class="th w-[260px]">"Item"</th>
                        <th class="th w-[80px] num">"Qty"</th>
                        <th class="th w-[120px] num">"Catalogue price"</th>
                        <th class="th w-[90px] num">"Discount %"</th>
                        <th class="th w-[120px] num">"Final rate"</th>
                        <th class="th w-[120px] num">"Amount"</th>
                        <th class="th w-[150px]">"Contents shown"</th>
                        <th class="th w-[190px]">"Packaging"</th>
                        {can_edit.then(|| view! { <th class="th w-[40px]"></th> })}
                    </tr></thead>
                    <tbody>
                        <For each=move || keys.get() key=|k| *k let:key>
                            <LineRow key=key lines=lines opts=opts packaging=packaging settings=settings can_edit=can_edit />
                        </For>
                    </tbody>
                </table>
            </div>
        </Show>
    }
}

/// Choices from settings, keeping the line's current value if removed since.
fn choices(list: &[String], current: &str) -> Vec<String> {
    let mut out: Vec<String> = list.iter().filter(|s| !s.is_empty()).cloned().collect();
    if !current.is_empty() && !out.iter().any(|s| s == current) {
        out.push(current.to_string());
    }
    out
}

#[component]
fn LineRow(
    key: u32,
    lines: RwSignal<Vec<Line>>,
    opts: StoredValue<QuoteOptions>,
    packaging: Memo<Vec<String>>,
    settings: StoredValue<Settings>,
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
    let priced = Memo::new(move |_| price_quote_line(&line.with(Line::priced)));
    let has_hamper = Memo::new(move |_| line.with(|l| l.hamper_id.is_some()));
    let contents = move || {
        let l = line.get();
        let items = l
            .hamper_id
            .as_ref()
            .and_then(|id| opts.with_value(|o| o.hampers.iter().find(|h| h.id == *id).map(|h| h.items.clone())))
            .unwrap_or_default();
        view! { <HamperContents class="text-xs" items=items detail_mode=l.detail_mode packaging_treatment=l.packaging_treatment packaging=packaging.get() /> }
    };
    let number = move |label: &'static str, get: fn(&Line) -> String, set: fn(&mut Line, String)| {
        view! {
            <td class="td">
                <input aria-label=label inputmode="decimal" class="input input-num" disabled=!can_edit prop:value=move || line.with(get)
                    on:input=move |e| { let v = event_target_value(&e); update(&|l| set(l, v.clone())); } />
            </td>
        }
    };
    let pick = move |label: &'static str,
                     list: fn(&Settings) -> Vec<String>,
                     get: fn(&Line) -> String,
                     set: fn(&mut Line, String)| {
        view! {
            <td class="td">
                <select aria-label=label class="select" disabled=!can_edit on:change=move |e| { let v = event_target_value(&e); update(&|l| set(l, v.clone())); }>
                    {move || {
                        let current = line.with(get);
                        choices(&settings.with_value(list), &current).into_iter().map(|m| {
                            let on = m == current;
                            let text = m.clone();
                            view! { <option value=m selected=on>{text}</option> }
                        }).collect_view()
                    }}
                </select>
            </td>
        }
    };
    view! {
        <tr class="[&>td]:align-top">
            <td class="td">
                <div class="font-medium">{move || line.with(|l| l.hamper_name.clone())}</div>
                <div class="font-mono text-xs text-(--color-muted)">{move || line.with(|l| l.hamper_code.clone())}</div>
                // A read-only preview of what the printed document will list.
                {contents}
            </td>
            {number("Quantity", |l| l.qty.clone(), |l, v| l.qty = v)}
            {number("Catalogue price", |l| l.catalogue_price.clone(), |l, v| l.catalogue_price = v)}
            {number("Discount percent", |l| l.discount_pct.clone(), |l, v| l.discount_pct = v)}
            <td class="td num text-(--color-muted)">{move || format_money(Some(priced.get().final_rate))}</td>
            <td class="td num font-medium">{move || format_money(Some(priced.get().amount))}</td>
            {move || if has_hamper.get() {
                view! {
                    {pick("Contents shown", |s| s.detail_modes.clone(), |l| l.detail_mode.clone(), |l, v| l.detail_mode = v)}
                    {pick("Packaging treatment", |s| s.packaging_treatments.clone(), |l| l.packaging_treatment.clone(), |l, v| l.packaging_treatment = v)}
                }.into_any()
            } else {
                // Product lines have no hamper: no contents or packaging to set.
                view! { <td colspan="2" class="td text-xs text-(--color-muted)">"Single product"</td> }.into_any()
            }}
            {can_edit.then(|| view! {
                <td class="td num">
                    <button type="button" aria-label=move || format!("Remove {}", line.with(|l| l.hamper_code.clone()))
                        class="rounded px-1.5 py-0.5 text-(--color-muted) hover:bg-red-50 hover:text-red-700"
                        on:click=move |_| lines.update(|ls| ls.retain(|l| l.key != key))>"×"</button>
                </td>
            })}
        </tr>
    }
}

#[component]
fn Row(label: Signal<String>, #[prop(into)] value: Signal<String>, #[prop(optional)] strong: bool) -> impl IntoView {
    view! {
        <div class="flex items-baseline justify-between gap-4">
            <dt class="text-(--color-muted)">{label}</dt>
            <dd class=if strong { "tabular-nums text-base font-semibold" } else { "tabular-nums font-medium" }>
                {move || { let v = value.get(); if v.is_empty() { "—".to_string() } else { v } }}
            </dd>
        </div>
    }
}

/// Notes and terms beside the order total, discount, charges and GST.
#[component]
fn Totals(
    fields: Fields,
    totals: Memo<QuotePricing>,
    option_totals: Memo<Vec<(String, QuotePricing)>>,
    combined: Memo<bool>,
    can_edit: bool,
) -> impl IntoView {
    let off = !can_edit;
    let amount = move |id: &'static str, label: &'static str, value: RwSignal<String>, class: &'static str| {
        view! { <TextField id=id label=label value=value disabled=off mode="decimal" class=if class.is_empty() { "input-num" } else { class } /> }
    };
    let money = move |pick: fn(&QuotePricing) -> Option<Decimal>| {
        Signal::derive(move || totals.with(pick).map(|v| format_money(Some(v))).unwrap_or_default())
    };
    let label = |s: &'static str| Signal::stored(s.to_string());
    view! {
        <section class="grid gap-4 lg:grid-cols-2">
            <div class="card p-4">
                <h2 class="text-sm font-semibold">"Notes and terms"</h2>
                <div class="mt-3 space-y-3">
                    <Field label="Internal notes" id="notes">
                        <textarea id="notes" rows="2" class="input mt-1" disabled=off prop:value=fields.notes on:input=move |e| fields.notes.set(event_target_value(&e))></textarea>
                    </Field>
                    <Field label="Terms printed on the quotation" id="terms">
                        <textarea id="terms" rows="5" class="input mt-1" disabled=off prop:value=fields.terms on:input=move |e| fields.terms.set(event_target_value(&e))></textarea>
                    </Field>
                </div>
            </div>
            <div class="card p-4">
                <h2 class="text-sm font-semibold">{move || if combined.get() { "Order total" } else { "Option totals" }}</h2>
                <Show
                    when=move || combined.get()
                    fallback=move || view! {
                        <p class="mt-1 text-xs text-(--color-muted)">
                            "Each option is totalled on its own; the client picks one. Order discount and charges apply to combined orders only."
                        </p>
                        <div class="mt-3">{amount("gstRate", "GST %", fields.gst_rate, "input-num max-w-[120px]")}</div>
                        <dl class="mt-4 space-y-1.5 border-t border-(--color-line) pt-3 text-sm">
                            {move || option_totals.get().into_iter().map(|(l, t)| view! {
                                <Row label=Signal::stored(format!("{l} (incl. GST)")) value=t.grand_total.map(|v| format_money(Some(v))).unwrap_or_default() strong=true />
                            }).collect_view()}
                        </dl>
                    }
                >
                    <dl class="mt-3 space-y-1.5 text-sm"><Row label=label("Subtotal") value=money(|t| Some(t.subtotal)) /></dl>
                    <div class="mt-3 grid gap-3 sm:grid-cols-3">
                        {amount("orderDiscount", "Order discount", fields.order_discount, "")}
                        {amount("adj1", "Packaging / freight", fields.adj1, "")}
                        {amount("adj2", "Other charges", fields.adj2, "")}
                    </div>
                    <dl class="mt-4 space-y-1.5 text-sm"><Row label=label("Taxable value") value=money(|t| t.taxable_value) /></dl>
                    <div class="mt-3">{amount("gstRate", "GST %", fields.gst_rate, "input-num max-w-[120px]")}</div>
                    <dl class="mt-4 space-y-1.5 border-t border-(--color-line) pt-3 text-sm">
                        <Row label=label("GST") value=money(|t| t.gst_amount) />
                        <Row label=label("Grand total") value=money(|t| t.grand_total) strong=true />
                    </dl>
                </Show>
            </div>
        </section>
    }
}

/// Print, convert and delete for a saved quotation. An option-based quotation
/// converts the option the client chose into a combined-order invoice.
#[component]
fn Actions(
    quote: Quote,
    can_edit: bool,
    options: Vec<String>,
    busy: RwSignal<Option<&'static str>>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let go = move |to: String| navigate.with_value(|nav| nav(&to, Default::default()));
    let chosen =
        RwSignal::new(if options.len() == 1 { options.first().cloned().unwrap_or_default() } else { String::new() });
    let (doc_no, id) = (quote.doc_no.clone(), StoredValue::new(quote.id.clone()));
    let doc = StoredValue::new(doc_no.clone());
    let convert = move || {
        let (doc_no, option) = (doc.get_value(), chosen.get_untracked());
        error.set(None);
        busy.set(Some("convert"));
        spawn_local(async move {
            match convert_to_proforma(&doc_no, &option).await {
                Ok(new) => {
                    db::cache_drop(&["quotes", "dashboard"]).await;
                    let enc = String::from(js_sys::encode_uri_component(&doc_no));
                    go(format!("{}?converted={enc}", quote_href(&new)));
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };
    let delete = move || {
        let doc_no = doc.get_value();
        if !confirm(&format!("Delete {doc_no}? This cannot be undone.")) {
            return;
        }
        let id = id.get_value();
        error.set(None);
        busy.set(Some("delete"));
        spawn_local(async move {
            match Q::from("quotes").eq("id", &id).delete().await {
                Ok(_) => {
                    db::cache_drop(&["quotes", "dashboard", &format!("quote:{doc_no}")]).await;
                    go("/quotes?deleted=1".into());
                }
                Err(e) => {
                    busy.set(None);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };
    let is_quotation = quote.doc_type == DocType::Quotation;
    let many = options.len() > 1;
    view! {
        <div class="mt-4 flex flex-wrap items-center gap-2">
            <A href=format!("{}/print", quote_href(&doc_no)) attr:class="btn-secondary">"Print / PDF"</A>
            {(can_edit && is_quotation).then(|| view! {
                <div class="flex flex-wrap items-center gap-2">
                    {many.then(|| view! {
                        <select aria-label="Option the client chose" class="select" on:change=move |e| chosen.set(event_target_value(&e))>
                            <option value="" disabled=true selected=true>"Client chose…"</option>
                            {options.iter().map(|o| view! { <option value=o.clone()>{o.clone()}</option> }).collect_view()}
                        </select>
                    })}
                    <button type="button" class="btn-secondary" disabled=move || busy.get().is_some() on:click=move |_| convert()>
                        {move || if busy.get() == Some("convert") { "Converting…" } else { "Convert to proforma" }}
                    </button>
                </div>
            })}
            {can_edit.then(|| view! {
                <button type="button" class="btn-danger ml-auto" disabled=move || busy.get().is_some() on:click=move |_| delete()>
                    {move || if busy.get() == Some("delete") { "Deleting…" } else { "Delete quote" }}
                </button>
            })}
        </div>
    }
}

/// Turns an accepted quotation into a proforma invoice: a new document with
/// its own number, linked back. From an option-based quotation only the chosen
/// option is carried across, as a combined order: those are the confirmed items.
async fn convert_to_proforma(doc_no: &str, option: &str) -> Result<String, ApiError> {
    let read = Q::from("quotes").select("*,quote_items(*)").eq("doc_no", doc_no);
    let (quote, s) = join!(read.one::<Value>(), crate::data::load_settings());
    let Some(Value::Object(mut rest)) = quote? else { return Err(ApiError::NotFound("Not found".into())) };
    let pi_terms = s?.pi_terms;
    let all: Vec<QuoteItem> =
        rest.remove("quote_items").map(serde_json::from_value).transpose().ok().flatten().unwrap_or_default();
    for k in ["id", "doc_no", "created_at", "updated_at"] {
        rest.remove(k);
    }
    let structure = rest.get("quote_structure").and_then(Value::as_str).unwrap_or_default().to_string();
    let is_options = structure != COMBINED_ORDER;
    if is_options && option.is_empty() {
        return Err(ApiError::Invalid("Choose the option the client picked.".into()));
    }
    let lines: Vec<&QuoteItem> = if is_options {
        all.iter().filter(|l| option_key(l.option_label.as_deref()) == option).collect()
    } else {
        all.iter().collect()
    };
    if lines.is_empty() {
        return Err(ApiError::Invalid(if is_options {
            format!("{option} has no items to invoice.")
        } else {
            "This quotation has no items.".into()
        }));
    }
    rest.insert("doc_type".into(), json!("proforma_invoice"));
    rest.insert("quote_structure".into(), json!(COMBINED_ORDER));
    if is_options {
        // An option is priced without order discount or charges; keep the invoice equal to it.
        for k in ["order_discount", "adj1", "adj2"] {
            rest.insert(k.into(), json!(0));
        }
    }
    rest.insert("status".into(), json!("Draft"));
    rest.insert("linked_doc_no".into(), json!(doc_no));
    // A PI carries payment terms (advance, delivery), not quotation terms.
    if !pi_terms.is_empty() {
        rest.insert("terms".into(), json!(pi_terms));
    }
    let f = lattice_core::js::f;
    rest.insert(
        "lines".into(),
        Value::Array(
            lines
                .iter()
                .map(|l| {
                    json!({
                        "option_label": null, "hamper_id": l.hamper_id, "hamper_code": l.hamper_code, "hamper_name": l.hamper_name,
                        "qty": n(f(l.qty)), "catalogue_price": n(f(l.catalogue_price)), "discount_pct": n(f(l.discount_pct)),
                        "detail_mode": l.detail_mode, "packaging_treatment": l.packaging_treatment,
                    })
                })
                .collect(),
        ),
    );
    db::rpc("save_quote", &json!({ "p": Value::Object(rest) })).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_option_skips_taken_labels() {
        let taken = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(next_option_label(&taken(&["Option 1"])), "Option 2");
        assert_eq!(next_option_label(&taken(&["Option 1", "Option 3"])), "Option 4");
        assert_eq!(next_option_label(&taken(&["Gold", "Option 3"])), "Option 4");
    }

    #[test]
    fn choices_keep_a_retired_value() {
        let list = ["A".to_string(), "B".to_string()];
        assert_eq!(choices(&list, "C"), ["A", "B", "C"]);
        assert_eq!(choices(&list, ""), ["A", "B"]);
    }
}
