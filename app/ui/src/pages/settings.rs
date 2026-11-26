//! Settings (port of `(app)/settings/*`), admin only. The tab lives in the
//! URL (`?tab=users`), so it survives a refresh and can be linked to.

use crate::components::PageHeader;
use crate::data::{categories_query, use_settings};
use crate::db::{self, Q, insert, live, me};
use crate::kit::{confirm, pct_str};
use crate::pages::rates::RatesMaster;
use lattice_core::api::ApiError;
use lattice_core::colors::{code_clash, derive_category_code};
use lattice_core::js::f;
use lattice_core::numbers::normalize_pct_str;
use lattice_core::records::{Category, Profile, UserRole};
use lattice_core::settings::Settings;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;
use serde_json::{Value, json};
use std::time::Duration;

// Most-used first; the first is what /settings opens on.
static TABS: [(&str, &str, &str); 7] = [
    ("rates", "Rates", "Material, hardware, finishing and machine rates used to cost products."),
    ("dropdowns", "Dropdowns", "The options people pick from across the app: occasions, vendors, colours, statuses."),
    ("categories", "Categories", "Product categories, and whether each counts towards a hamper's item count."),
    ("tools", "Tools", "Recycle Bin, price refresh and app updates."),
    ("users", "Users", "Who can sign in and what they can do."),
    ("company", "Company & terms", "Your letterhead details, GST and the terms printed on quotes and invoices."),
    ("keys", "Key bindings", "Shortcuts and mouse buttons for the app, like a game's controls."),
];

async fn save_setting(key: &str, value: Value) -> Result<(), ApiError> {
    insert("app_settings", &json!({ "key": key, "value": value }), Some("key"), None).await?;
    db::cache_drop(&["settings"]).await;
    Ok(())
}

/// A save's outcome beside its button: errors stay, "Saved" fades.
#[derive(Clone, Copy)]
struct Status {
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    saved: RwSignal<bool>,
}

impl Status {
    fn new() -> Self {
        Self { busy: RwSignal::new(false), error: RwSignal::new(None), saved: RwSignal::new(false) }
    }

    fn start(&self) {
        self.busy.set(true);
        self.error.set(None);
        self.saved.set(false);
    }

    fn done(&self, res: Result<(), ApiError>) {
        self.busy.set(false);
        match res {
            Ok(()) => {
                self.saved.set(true);
                let saved = self.saved;
                set_timeout(move || saved.set(false), Duration::from_secs(3));
            }
            Err(e) => self.error.set(Some(e.to_string())),
        }
    }

    fn fail(&self, msg: impl Into<String>) {
        self.error.set(Some(msg.into()));
    }

    fn view(self) -> impl IntoView {
        view! {
            {move || self.error.get().map(|e| view! { <span role="alert" class="text-sm text-red-700">{e}</span> })}
            <Show when=move || self.saved.get()><span role="status" class="text-sm text-emerald-700">"Saved"</span></Show>
        }
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    if !me().role.is_admin() {
        return view! {
            <PageHeader title="Settings">""</PageHeader>
            <div class="card p-6 text-sm text-(--color-muted)">"Only an admin can change settings."</div>
        }
        .into_any();
    }
    let query = use_query_map();
    let tab = Memo::new(move |_| {
        let t = query.with(|q| q.get("tab")).unwrap_or_default();
        TABS.iter().find(|(k, _, _)| *k == t).unwrap_or(&TABS[0]).0
    });
    view! {
        <PageHeader title="Settings">""</PageHeader>
        <nav aria-label="Settings sections" class="flex flex-wrap gap-1 rounded-lg bg-slate-100 p-1">
            {TABS.iter().enumerate().map(|(i, (key, label, _))| {
                let href = if i == 0 { "/settings".to_string() } else { format!("/settings?tab={key}") };
                view! {
                    <A href=href attr:aria-current=move || (tab.get() == *key).then_some("page")
                        attr:class=move || format!("rounded-md px-3 py-1.5 text-sm font-semibold transition-all {}",
                            if tab.get() == *key { "bg-white text-(--color-brand-dark) shadow-sm" } else { "text-(--color-muted) hover:text-(--color-ink)" })>
                        {*label}
                    </A>
                }
            }).collect_view()}
        </nav>
        <p class="mt-2 mb-4 text-sm text-(--color-muted)">{move || TABS.iter().find(|(k, _, _)| *k == tab.get()).map(|t| t.2)}</p>
        {move || match tab.get() {
            "dropdowns" => view! { <section class="card p-4"><PickLists /></section> }.into_any(),
            "categories" => view! { <Categories /> }.into_any(),
            "tools" => view! { <Tools /> }.into_any(),
            "users" => view! { <Users /> }.into_any(),
            "company" => view! { <CompanyTab /> }.into_any(),
            "keys" => view! { <crate::pages::keys::KeyBindingsTab /> }.into_any(),
            _ => view! { <RatesMaster embedded=true /> }.into_any(),
        }}
    }
    .into_any()
}

/// Waits for settings, then renders `child` once with them (forms own what is typed).
fn with_settings<V: IntoView + 'static>(child: impl Fn(Settings) -> V + Send + Sync + 'static) -> impl IntoView {
    let child = std::sync::Arc::new(child);
    let settings = use_settings();
    let first = RwSignal::new(None::<Settings>);
    Effect::new(move |_| {
        if first.with_untracked(Option::is_none)
            && let Some(s) = settings.data.get()
        {
            first.set(Some(s));
        }
    });
    view! {
        <Show when=move || first.with(Option::is_some) fallback=|| view! { <div class="card h-64 animate-pulse bg-white/60"></div> }>
            {let child = child.clone(); move || first.get_untracked().map(|s| child(s))}
        </Show>
    }
}

// ------------------------------------------------------------- company & terms

#[component]
fn CompanyTab() -> impl IntoView {
    with_settings(|s| {
        // Terms change more often than the letterhead, so they come first.
        view! {
            <div class="grid gap-4 lg:grid-cols-2">
                <div class="grid content-start gap-4">
                    <TermsForm key="quote_terms" title="Quote terms" terms=s.quote_terms.clone() />
                    <TermsForm key="pi_terms" title="Proforma invoice terms" terms=s.pi_terms.clone() />
                </div>
                <CompanyForm s=s />
            </div>
        }
    })
}

#[component]
fn TermsForm(key: &'static str, title: &'static str, terms: String) -> impl IntoView {
    let value = RwSignal::new(terms);
    let status = Status::new();
    let save = move || {
        status.start();
        spawn_local(async move { status.done(save_setting(key, json!(value.get_untracked())).await) });
    };
    view! {
        <div class="card p-4">
            <h2 class="text-sm font-semibold">{title}</h2>
            <p class="mt-0.5 text-xs text-(--color-muted)">"Copied into each new document, where it can still be edited."</p>
            <textarea rows="6" class="input mt-3" aria-label=title prop:value=value on:input=move |e| value.set(event_target_value(&e))></textarea>
            <div class="mt-3 flex items-center gap-3">
                <button type="button" class="btn-primary" disabled=move || status.busy.get() on:click=move |_| save()>
                    {move || if status.busy.get() { "Saving…" } else { "Save" }}
                </button>
                {status.view()}
            </div>
        </div>
    }
}

#[component]
fn CompanyForm(s: Settings) -> impl IntoView {
    let c = s.company;
    let fields: [(&'static str, &'static str, RwSignal<String>, &'static str); 11] = [
        ("name", "Company name", RwSignal::new(c.name), ""),
        ("legal_name", "Legal name (on invoices)", RwSignal::new(c.legal_name), ""),
        ("address", "Address", RwSignal::new(c.address), "sm:col-span-2"),
        ("gstin", "GSTIN", RwSignal::new(c.gstin), "font-mono"),
        ("phone", "Phone", RwSignal::new(c.phone), ""),
        ("email", "Email", RwSignal::new(c.email), ""),
        ("website", "Website", RwSignal::new(c.website), ""),
        ("bank_account_name", "Account name", RwSignal::new(c.bank_account_name), ""),
        ("bank_name", "Bank", RwSignal::new(c.bank_name), ""),
        ("bank_account_no", "Account number", RwSignal::new(c.bank_account_no), "font-mono"),
        ("bank_ifsc", "IFSC code", RwSignal::new(c.bank_ifsc), "font-mono uppercase"),
    ];
    let gst = RwSignal::new(pct_str(s.gst_rate));
    let status = Status::new();
    let save = move || {
        let mut company = serde_json::Map::new();
        for (key, _, v, _) in fields {
            let t = v.get_untracked().trim().to_string();
            company.insert(key.into(), json!(if key == "bank_ifsc" { t.to_uppercase() } else { t }));
        }
        if company.get("name").and_then(Value::as_str).is_none_or(str::is_empty) {
            return status.fail("Company name is required");
        }
        let rate = f(normalize_pct_str(&gst.get_untracked()));
        status.start();
        spawn_local(async move {
            let res = match save_setting("company", Value::Object(company)).await {
                Ok(()) => save_setting("gst_rate", crate::kit::json_num(rate)).await,
                e => e,
            };
            status.done(res);
        });
    };
    let input = move |(key, label, value, class): (&'static str, &'static str, RwSignal<String>, &'static str)| {
        let id = format!("company-{key}");
        if key == "address" {
            return view! {
                <div class="sm:col-span-2">
                    <label class="label" for=id.clone()>{label}</label>
                    <textarea id=id rows="2" class="input mt-1" prop:value=value on:input=move |e| value.set(event_target_value(&e))></textarea>
                </div>
            }
            .into_any();
        }
        view! {
            <div>
                <label class="label" for=id.clone()>{label}</label>
                <input id=id class=format!("input mt-1 {class}") placeholder=if key == "legal_name" { "e.g. Palm Length LLP" } else { "" }
                    prop:value=value on:input=move |e| value.set(event_target_value(&e)) />
            </div>
        }
        .into_any()
    };
    view! {
        <div class="card p-4">
            <h2 class="text-sm font-semibold">"Company details"</h2>
            <p class="mt-0.5 text-xs text-(--color-muted)">"Printed at the top of every quotation and proforma invoice."</p>
            <div class="mt-4 grid gap-3 sm:grid-cols-2">
                {fields.into_iter().take(7).map(input).collect_view()}
                <div>
                    <label class="label" for="gst_rate">"Default GST %"</label>
                    <input id="gst_rate" inputmode="decimal" class="input input-num mt-1" prop:value=gst on:input=move |e| gst.set(event_target_value(&e)) />
                </div>
                <div class="border-t border-(--color-line) pt-3 sm:col-span-2">
                    <h3 class="text-xs font-semibold">"Bank details"</h3>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Printed on proforma invoices."</p>
                </div>
                {fields.into_iter().skip(7).map(input).collect_view()}
            </div>
            <div class="mt-4 flex items-center gap-3">
                <button type="button" class="btn-primary" disabled=move || status.busy.get() on:click=move |_| save()>
                    {move || if status.busy.get() { "Saving…" } else { "Save" }}
                </button>
                {status.view()}
            </div>
        </div>
    }
}

// ------------------------------------------------------------------ pick-lists

// Most-used first: the first is open when the tab loads.
static LISTS: [(&str, &str, &str); 9] = [
    ("collections", "Occasions / collections", "Used to group hampers, e.g. Festive, Corporate, Wedding."),
    ("sources", "Vendors / sources", "Where a product comes from, e.g. In-house, Outsourced."),
    ("product_colors", "Colours / finishes", "Colour options offered for products."),
    ("hamper_statuses", "Hamper statuses", "e.g. Draft, Approved, Active, Discontinued."),
    ("quote_statuses", "Quote statuses", "Only managers can move a quote between these."),
    ("validity_options", "Quote validity", "How long a quote is valid, e.g. 7 Days, 15 Days."),
    (
        "packaging_treatments",
        "Packaging treatments",
        "A name containing \"absorb\" folds packaging cost into the hamper.",
    ),
    (
        "detail_modes",
        "Contents shown on quotes",
        "A name containing \"hide\" hides contents; \"summary\" prints a count.",
    ),
    ("quote_structures", "Quote structures", "\"Combined Order\" has one total; any other is priced per option."),
];

fn list_of(s: &Settings, key: &str) -> Vec<String> {
    match key {
        "collections" => s.collections.clone(),
        "sources" => s.sources.clone(),
        "product_colors" => s.product_colors.clone(),
        "hamper_statuses" => s.hamper_statuses.clone(),
        "quote_statuses" => s.quote_statuses.clone(),
        "validity_options" => s.validity_options.clone(),
        "packaging_treatments" => s.packaging_treatments.clone(),
        "detail_modes" => s.detail_modes.clone(),
        "quote_structures" => s.quote_structures.clone(),
        _ => Vec::new(),
    }
}

/// The stored list, read fresh so two people editing don't undo each other.
/// A list never saved yet is still on its defaults, which `shown` holds.
async fn stored_list(key: &str, shown: Vec<String>) -> Result<Vec<String>, ApiError> {
    #[derive(serde::Deserialize)]
    struct Row {
        value: Value,
    }
    let row = Q::from("app_settings").select("value").eq("key", key).one::<Row>().await?;
    Ok(row.and_then(|r| serde_json::from_value(r.value).ok()).unwrap_or(shown))
}

#[component]
fn PickLists() -> impl IntoView {
    with_settings(|s| {
        let lists = RwSignal::new(LISTS.iter().map(|(k, _, _)| (*k, list_of(&s, k))).collect::<Vec<_>>());
        let selected = RwSignal::new(LISTS[0].0);
        let search = RwSignal::new(String::new());
        let adding = RwSignal::new(None::<String>);
        let message = RwSignal::new(None::<(bool, String)>);
        let pending = RwSignal::new(false);
        let config = move || LISTS.iter().find(|(k, _, _)| *k == selected.get()).copied().unwrap_or(LISTS[0]);
        let items = move || {
            lists.with(|l| l.iter().find(|(k, _)| *k == selected.get()).map(|(_, v)| v.clone()).unwrap_or_default())
        };
        let set_items = move |key: &str, v: Vec<String>| {
            lists.update(|l| {
                if let Some(entry) = l.iter_mut().find(|(k, _)| *k == key) {
                    entry.1 = v;
                }
            });
        };
        let notify = move |ok: bool, text: String| {
            message.set(Some((ok, text.clone())));
            set_timeout(
                move || {
                    message.update(|m| {
                        if m.as_ref().is_some_and(|(_, t)| *t == text) {
                            *m = None
                        }
                    })
                },
                Duration::from_secs(4),
            );
        };
        // Applies `next` at once, saves, and rolls back if the save fails.
        let change = move |item: String, adding_it: bool| {
            let (key, label) = (selected.get_untracked(), config().1);
            let previous = items();
            let next: Vec<String> = if adding_it {
                previous.iter().cloned().chain([item.clone()]).collect()
            } else {
                previous.iter().filter(|i| **i != item).cloned().collect()
            };
            set_items(key, next);
            pending.set(true);
            spawn_local(async move {
                let res: Result<Vec<String>, ApiError> = async {
                    let current = stored_list(key, previous.clone()).await?;
                    let updated: Vec<String> = if adding_it {
                        if current.iter().any(|v| v.to_lowercase() == item.to_lowercase()) {
                            return Err(ApiError::Invalid(format!("\"{item}\" already exists in this pick-list.")));
                        }
                        current.into_iter().chain([item.clone()]).collect()
                    } else {
                        current.into_iter().filter(|v| *v != item).collect()
                    };
                    // A colour's code ends product codes, so two colours can never share one.
                    if key == "product_colors"
                        && let Some(c) = code_clash(&updated)
                    {
                        return Err(ApiError::Invalid(format!(
                            "\"{}\" would get code {}, already used by {}. Pick a different name.",
                            c.name, c.code, c.with
                        )));
                    }
                    save_setting(key, json!(updated)).await?;
                    Ok(updated)
                }
                .await;
                pending.set(false);
                match res {
                    Ok(updated) => {
                        set_items(key, updated);
                        notify(
                            true,
                            if adding_it {
                                format!("Added \"{item}\" to {label}.")
                            } else {
                                format!("Removed \"{item}\" from {label}.")
                            },
                        );
                    }
                    Err(e) => {
                        set_items(key, previous);
                        notify(false, e.to_string());
                    }
                }
            });
        };
        let filtered = move || {
            let q = search.get().trim().to_lowercase();
            items().into_iter().filter(|i| q.is_empty() || i.to_lowercase().contains(&q)).collect::<Vec<_>>()
        };
        let exact = move || {
            let q = search.get().trim().to_lowercase();
            items().iter().any(|i| i.to_lowercase() == q)
        };
        let open_add = move |prefill: Option<String>| {
            adding.set(Some(prefill.unwrap_or_else(|| search.get_untracked()).trim().to_string()))
        };
        let values = |n: usize| format!("{n} {}", if n == 1 { "value" } else { "values" });
        let defaults = [
            ("default_detail_mode", "Default contents setting", s.default_detail_mode.clone(), "detail_modes"),
            (
                "default_packaging_treatment",
                "Default packaging treatment",
                s.default_packaging_treatment.clone(),
                "packaging_treatments",
            ),
            ("default_validity", "Default validity", s.default_validity.clone(), "validity_options"),
        ];

        view! {
            <div class="space-y-4">
                <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    <div class="sm:col-span-2">
                        <label class="label" for="picklist-selector">"Select Pick-list"</label>
                        <div class="mt-1 flex items-center gap-2">
                            <select id="picklist-selector" class="select flex-1 font-medium" on:change=move |e| {
                                let v = event_target_value(&e);
                                if let Some((k, _, _)) = LISTS.iter().find(|(k, _, _)| *k == v) {
                                    selected.set(k);
                                    search.set(String::new());
                                }
                            }>
                                {LISTS.iter().map(|(k, label, _)| view! {
                                    <option value=*k selected=move || selected.get() == *k>
                                        {move || format!("{label} ({})", values(lists.with(|l| l.iter().find(|(x, _)| x == k).map_or(0, |(_, v)| v.len()))))}
                                    </option>
                                }).collect_view()}
                            </select>
                            <button type="button" class="btn-primary shrink-0" title=move || format!("Add new item to {}", config().1) on:click=move |_| open_add(None)>"+ Add new"</button>
                        </div>
                        <p class="mt-1 text-xs text-(--color-muted)">{move || config().2}</p>
                    </div>
                </div>
                <div class="relative max-w-md">
                    <label class="label" for="picklist-search">"Search options"</label>
                    <input id="picklist-search" type="search" class="input mt-1" placeholder=move || format!("Search in {}...", config().1.to_lowercase())
                        prop:value=search on:input=move |e| search.set(event_target_value(&e))
                        on:keydown=move |e| if e.key() == "Enter" && !search.get_untracked().trim().is_empty() && !exact() { e.prevent_default(); open_add(None); } />
                </div>
                <Show when=move || !search.get().trim().is_empty() && !exact()>
                    <div class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-(--color-line) bg-(--color-paper) px-3.5 py-2 text-xs">
                        <span class="text-(--color-muted)">"Not found in this list: "<strong>{move || format!("“{}”", search.get().trim())}</strong></span>
                        <button type="button" class="font-semibold text-(--color-brand) hover:underline" on:click=move |_| open_add(None)>
                            {move || format!("+ Add “{}” as new option", search.get().trim())}
                        </button>
                    </div>
                </Show>
                {move || message.get().map(|(ok, text)| view! {
                    <div role="status" class=if ok { "rounded-lg border border-emerald-200 bg-emerald-50 px-3.5 py-2 text-xs font-medium text-emerald-800" } else { "rounded-lg border border-red-200 bg-red-50 px-3.5 py-2 text-xs font-medium text-red-800" }>{text}</div>
                })}
                <div class="rounded-lg border border-(--color-line) bg-white p-3.5">
                    <div class="mb-2.5 flex items-center justify-between text-xs text-(--color-muted)">
                        <span>{move || {
                            let (n, total, q) = (filtered().len(), items().len(), search.get().trim().to_string());
                            let base = format!("{n} {}", if n == 1 { "item" } else { "items" });
                            if q.is_empty() { base } else { format!("{base} matching \"{q}\" (out of {total})") }
                        }}</span>
                        <Show when=move || pending.get()><span class="text-(--color-brand)">"Saving changes…"</span></Show>
                    </div>
                    {move || {
                        let list = filtered();
                        if list.is_empty() {
                            let q = search.get().trim().to_string();
                            return if q.is_empty() {
                                view! {
                                    <div class="py-8 text-center text-sm">
                                        <p class="text-(--color-muted)">{format!("No options in {} yet.", config().1)}</p>
                                        <button type="button" class="btn-secondary mt-3" on:click=move |_| open_add(None)>"+ Add the first option"</button>
                                    </div>
                                }.into_any()
                            } else {
                                let prefill = q.clone();
                                view! {
                                    <div class="py-8 text-center text-sm">
                                        <p class="text-(--color-muted)">{format!("No matching options for “{q}”")}</p>
                                        <button type="button" class="btn-primary mt-3" on:click=move |_| open_add(Some(prefill.clone()))>{format!("+ Add “{q}” now")}</button>
                                    </div>
                                }.into_any()
                            };
                        }
                        view! {
                            <div class="flex flex-wrap gap-2">
                                {list.into_iter().map(|item| {
                                    let it = item.clone();
                                    view! {
                                        <div class="inline-flex items-center gap-1.5 rounded-full border border-(--color-line) bg-(--color-paper) py-1 pr-1.5 pl-3 text-xs text-(--color-ink) hover:border-slate-300 hover:bg-slate-100">
                                            <span class="font-medium">{item.clone()}</span>
                                            <button type="button" title=format!("Remove \"{item}\"") aria-label=format!("Remove {item}") disabled=move || pending.get()
                                                class="rounded-full px-1 text-(--color-muted) hover:bg-red-100 hover:text-red-700 disabled:opacity-40"
                                                on:click=move |_| change(it.clone(), false)>"×"</button>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>
                {move || adding.get().map(|initial| view! {
                    <AddOption label=config().1 initial=initial existing=items() pending=pending
                        on_add=move |v: String| { adding.set(None); search.set(String::new()); change(v, true); }
                        on_close=move || adding.set(None) />
                })}
                // Here, not on the page, so an option added above is offered straight away.
                <div class="mt-6 border-t border-(--color-line) pt-4">
                    <h3 class="text-xs font-semibold tracking-wider text-(--color-muted) uppercase">"Default Selections"</h3>
                    <div class="mt-3 grid gap-3 md:grid-cols-3">
                        {defaults.into_iter().map(|(key, label, value, list)| view! {
                            <DefaultForm key=key label=label value=value options=Signal::derive(move || lists.with(|l| l.iter().find(|(k, _)| *k == list).map(|(_, v)| v.clone()).unwrap_or_default())) />
                        }).collect_view()}
                    </div>
                </div>
            </div>
        }
    })
}

/// "Add new option": refuses blanks and duplicates before calling `on_add`.
#[component]
fn AddOption(
    label: &'static str,
    initial: String,
    existing: Vec<String>,
    pending: RwSignal<bool>,
    on_add: impl Fn(String) + Send + Sync + 'static,
    on_close: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let value = RwSignal::new(initial);
    let error = RwSignal::new(None::<String>);
    let input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
        }
    });
    let submit = move || {
        let t = value.get_untracked().trim().to_string();
        if t.is_empty() {
            return error.set(Some("Please enter an option name.".into()));
        }
        if existing.iter().any(|i| i.to_lowercase() == t.to_lowercase()) {
            return error.set(Some(format!("\"{t}\" already exists in {label}.")));
        }
        on_add(t);
    };
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs"
            on:click=move |e| if e.target() == e.current_target() { on_close() }
            on:keydown=move |e| if e.key() == "Escape" { on_close() }>
            <div role="dialog" aria-modal="true" aria-labelledby="add-modal-title" class="card w-full max-w-md p-5 shadow-xl">
                <div class="flex items-start justify-between border-b border-(--color-line) pb-3">
                    <div>
                        <h3 id="add-modal-title" class="text-base font-semibold text-(--color-ink)">"Add new option"</h3>
                        <p class="mt-0.5 text-xs text-(--color-muted)">"Adding to "<strong>{label}</strong></p>
                    </div>
                    <button type="button" aria-label="Close dialog" class="rounded-full px-2 text-(--color-muted) hover:bg-slate-100" on:click=move |_| on_close()>"×"</button>
                </div>
                <form class="mt-4 space-y-4" on:submit=move |e| { e.prevent_default(); submit(); }>
                    <div>
                        <label class="label" for="new-picklist-item">"Option value"</label>
                        <input node_ref=input id="new-picklist-item" class="input mt-1" placeholder="e.g. Festival Special" prop:value=value
                            on:input=move |e| { value.set(event_target_value(&e)); error.set(None); } />
                        {move || error.get().map(|e| view! { <p class="mt-1.5 text-xs text-red-600">{e}</p> })}
                    </div>
                    <div class="flex items-center justify-end gap-2 pt-2">
                        <button type="button" class="btn-secondary" disabled=move || pending.get() on:click=move |_| on_close()>"Cancel"</button>
                        <button type="submit" class="btn-primary" disabled=move || pending.get() || value.with(|v| v.trim().is_empty())>
                            {move || if pending.get() { "Saving…" } else { "+ Add option" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
fn DefaultForm(key: &'static str, label: &'static str, value: String, options: Signal<Vec<String>>) -> impl IntoView {
    let value = RwSignal::new(value);
    let status = Status::new();
    let save = move || {
        status.start();
        spawn_local(async move { status.done(save_setting(key, json!(value.get_untracked())).await) });
    };
    let id = format!("default-{key}");
    view! {
        <div class="rounded-md border border-(--color-line) p-3">
            <label class="label" for=id.clone()>{label}</label>
            <select id=id class="select mt-2" on:change=move |e| value.set(event_target_value(&e))>
                {move || {
                    let current = value.get();
                    let mut list: Vec<String> = options.get().into_iter().filter(|o| !o.is_empty()).collect();
                    if !current.is_empty() && !list.contains(&current) {
                        list.push(current.clone());
                    }
                    list.into_iter().map(|o| { let on = o == current; let text = o.clone(); view! { <option value=o selected=on>{text}</option> } }).collect_view()
                }}
            </select>
            <div class="mt-2 flex items-center gap-3">
                <button type="button" class="btn-secondary" disabled=move || status.busy.get() on:click=move |_| save()>
                    {move || if status.busy.get() { "Saving…" } else { "Save" }}
                </button>
                {status.view()}
            </div>
        </div>
    }
}

// ------------------------------------------------------------------ categories

#[component]
fn Categories() -> impl IntoView {
    let list = live::<Category>("categories", categories_query());
    // Re-rendered only when the list appears or empties, never on a refresh
    // (rows keep what is typed; `For` adds and drops rows by id).
    let state = Memo::new(move |_| list.data.with(|d| d.as_ref().map(Vec::is_empty)));
    view! {
        <section class="card overflow-hidden">
            <p class="border-b border-(--color-line) px-4 py-3 text-xs text-(--color-muted)">
                "Turn "<strong>"Counts as an item"</strong>" off for boxes, filler and ribbon: they add cost, but a client doesn’t count the box as one of the gifts."
            </p>
            {move || match state.get() {
                None => view! { <div class="h-40 animate-pulse bg-white/60"></div> }.into_any(),
                Some(true) => view! { <p class="px-4 py-6 text-sm text-(--color-muted)">"No categories yet. Import the Settings tab, or add them here."</p> }.into_any(),
                Some(false) => view! {
                    <div class="divide-y divide-(--color-line)">
                        <For each=move || list.data.get().unwrap_or_default() key=|c| c.id.clone() let:c>
                            <CategoryRow category=Some(c) list=list.data />
                        </For>
                    </div>
                }.into_any(),
            }}
            <div class="border-t border-(--color-line)">
                <CategoryRow category=None list=list.data />
            </div>
        </section>
    }
}

/// One category's row, or (`category` None) the add form at the bottom.
#[component]
fn CategoryRow(category: Option<Category>, list: RwSignal<Option<Vec<Category>>>) -> impl IntoView {
    let existing = category.is_some();
    let id = StoredValue::new(category.as_ref().map(|c| c.id.clone()));
    let sort_order = category.as_ref().map(|c| c.sort_order);
    let code = RwSignal::new(category.as_ref().and_then(|c| c.code.clone()).unwrap_or_default());
    let name = RwSignal::new(category.as_ref().map(|c| c.name.clone()).unwrap_or_default());
    let counts = RwSignal::new(category.as_ref().is_none_or(|c| c.counts_as_item));
    let status = Status::new();
    let deleting = RwSignal::new(false);
    let save = move || {
        let n = name.get_untracked().trim().to_string();
        if n.is_empty() {
            return status.fail("Category name is required.");
        }
        let raw = code.get_untracked().trim().to_uppercase();
        let values = json!({
            "name": n,
            "code": if raw.is_empty() { derive_category_code(&n) } else { raw },
            "counts_as_item": counts.get_untracked(),
            // A new one goes last.
            "sort_order": sort_order.unwrap_or_else(|| list.with_untracked(|c| c.iter().flatten().map(|c| c.sort_order).max().unwrap_or(-1) + 1)),
        });
        status.start();
        spawn_local(async move {
            let res: Result<Vec<Category>, ApiError> = match id.get_value() {
                Some(id) => Q::from("categories")
                    .eq("id", &id)
                    .update(&values, Some("*"))
                    .await
                    .and_then(|v| serde_json::from_value(v).map_err(|e| ApiError::Server(e.to_string()))),
                None => db::insert_returning("categories", &values).await,
            };
            match res {
                Ok(saved) => {
                    list.update(|l| {
                        if let Some(l) = l {
                            for c in saved {
                                match l.iter_mut().find(|x| x.id == c.id) {
                                    Some(x) => *x = c,
                                    None => l.push(c),
                                }
                            }
                        }
                    });
                    if !existing {
                        name.set(String::new());
                        code.set(String::new());
                        counts.set(true);
                    }
                    db::cache_drop(&["categories", "catalog", "products"]).await;
                    status.done(Ok(()));
                }
                Err(e) => status.done(Err(e)),
            }
        });
    };
    let delete = move || {
        let Some(target) = id.get_value() else { return };
        if !confirm(&format!("Delete the category {}?", name.get_untracked())) {
            return;
        }
        deleting.set(true);
        spawn_local(async move {
            let res = Q::from("categories").eq("id", &target).delete().await;
            deleting.set(false);
            match res {
                Ok(_) => {
                    list.update(|l| {
                        if let Some(l) = l {
                            l.retain(|c| c.id != target);
                        }
                    });
                    db::cache_drop(&["categories"]).await;
                }
                // Products reference categories, so a category in use cannot go.
                Err(e) => status.fail(format!("{e} Move its products to another category first.")),
            }
        });
    };
    view! {
        <div class=if existing { "flex flex-wrap items-center gap-2 px-2.5 py-1.5" } else { "flex flex-wrap items-center gap-2 p-3" }>
            <input aria-label="Category code" placeholder=if existing { "Code" } else { "Code (e.g. LC)" } title="2-letter category code used in product codes (e.g. LC, ED)"
                class=if existing { "input max-w-[70px] text-center font-mono text-xs font-bold uppercase" } else { "input max-w-[90px] text-center font-mono text-xs font-bold uppercase" }
                prop:value=code on:input=move |e| code.set(event_target_value(&e)) />
            <input aria-label=if existing { "Category name" } else { "New category name" } placeholder=if existing { "" } else { "New category name" }
                class="input max-w-[220px]" prop:value=name on:input=move |e| name.set(event_target_value(&e)) />
            <label class="flex items-center gap-1.5 whitespace-nowrap text-sm">
                <input type="checkbox" prop:checked=counts on:change=move |e| counts.set(event_target_checked(&e)) />
                "Counts as an item"
            </label>
            <button type="button" class=if existing { "btn-secondary" } else { "btn-primary" } disabled=move || status.busy.get() on:click=move |_| save()>
                {move || match (status.busy.get(), existing) {
                    (true, true) => "Saving…",
                    (true, false) => "Adding…",
                    (false, true) => "Save",
                    (false, false) => "Add category",
                }}
            </button>
            {existing.then(|| view! { <button type="button" class="btn-danger" disabled=move || deleting.get() on:click=move |_| delete()>"Delete"</button> })}
            {status.view()}
        </div>
    }
}

// ----------------------------------------------------------------------- tools

#[component]
fn Tools() -> impl IntoView {
    let tools = [
        ("/bin", "Recycle Bin", "Restore deleted hampers, products and photos. Kept for 30 days."),
        ("/admin/refresh-prices", "Refresh prices", "Update saved hampers with today's product prices."),
    ];
    view! {
        <section class="card divide-y divide-(--color-line)">
            {tools.into_iter().map(|(href, title, hint)| view! {
                <div class="flex flex-wrap items-center justify-between gap-3 p-4">
                    <div><h2 class="text-sm font-semibold">{title}</h2><p class="mt-0.5 text-xs text-(--color-muted)">{hint}</p></div>
                    <A href=href attr:class="btn-secondary">"Open"</A>
                </div>
            }).collect_view()}
            <crate::updates::UpdateRow />
            <div class="flex flex-wrap items-center justify-between gap-3 p-4">
                <div>
                    <h2 class="text-sm font-semibold">"Publish the app"</h2>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Builds the Windows app from the latest code and publishes it; everyone's app offers the update on its next launch."</p>
                </div>
                <crate::release::PublishRelease />
            </div>
            <div class="flex flex-wrap items-center justify-between gap-3 p-4">
                <div>
                    <h2 class="text-sm font-semibold">"Import old workbook "<span class="badge ml-1 align-middle">"One-time"</span></h2>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Moves products, hampers and quotes across from the old 6-tab spreadsheet."</p>
                </div>
                <A href="/admin/import" attr:class="btn-secondary">"Open"</A>
            </div>
        </section>
    }
}

// ----------------------------------------------------------------------- users

fn role_hint(r: UserRole) -> &'static str {
    match r {
        UserRole::Admin => "Everything, including products, settings and users",
        UserRole::Manager => "Hampers, all quotations, and quote status",
        UserRole::Sales => "Their own quotations",
    }
}

#[component]
fn Users() -> impl IntoView {
    let profiles = live::<Profile>("profiles", Q::from("profiles").select("*").order("created_at"));
    let self_id = me().id;
    view! {
        <section class="card overflow-hidden">
            <p class="border-b border-(--color-line) px-4 py-3 text-xs text-(--color-muted)">
                "New people sign up themselves and start as Sales; change their role here. Once your team is set up, turn off new sign-ups in Supabase (Authentication → Providers)."
            </p>
            <table class="w-full border-collapse text-sm">
                <thead><tr><th class="th">"Name"</th><th class="th">"Can access"</th><th class="th">"Role"</th></tr></thead>
                <tbody>
                    <For each=move || profiles.data.get().unwrap_or_default() key=|p| p.id.clone() let:p>
                        <UserRow is_self={p.id == self_id} profile=p />
                    </For>
                </tbody>
            </table>
        </section>
    }
}

#[component]
fn UserRow(profile: Profile, is_self: bool) -> impl IntoView {
    // The hint follows the saved role.
    let saved = RwSignal::new(profile.role);
    let picked = RwSignal::new(profile.role);
    let status = Status::new();
    let id = StoredValue::new(profile.id.clone());
    let save = move || {
        let role = picked.get_untracked();
        // Without this an admin could demote themselves and lock everyone out.
        if is_self && role != UserRole::Admin {
            return status.fail("You cannot remove your own admin access. Ask another admin to do it.");
        }
        status.start();
        spawn_local(async move {
            let res =
                Q::from("profiles").eq("id", id.get_value()).update(&json!({ "role": role }), None).await.map(|_| ());
            if res.is_ok() {
                saved.set(role);
                db::cache_drop(&["profiles"]).await;
            }
            status.done(res);
        });
    };
    let roles = [(UserRole::Sales, "Sales"), (UserRole::Manager, "Manager"), (UserRole::Admin, "Admin")];
    view! {
        <tr>
            <td class="td">{profile.full_name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "—".into())}{is_self.then(|| view! { <span class="badge ml-2">"you"</span> })}</td>
            <td class="td text-(--color-muted)">{move || role_hint(saved.get())}</td>
            <td class="td">
                <div class="flex items-center gap-2">
                    <select aria-label="Role" class="select max-w-[140px]" on:change=move |e| {
                        let v = event_target_value(&e);
                        if let Some((r, _)) = roles.iter().find(|(_, l)| l.eq_ignore_ascii_case(&v)) {
                            picked.set(*r);
                        }
                    }>
                        {roles.map(|(r, label)| view! { <option value=label selected={r == profile.role}>{label}</option> }).collect_view()}
                    </select>
                    <button type="button" class="btn-secondary" disabled=move || status.busy.get() on:click=move |_| save()>
                        {move || if status.busy.get() { "Saving…" } else { "Save" }}
                    </button>
                    {status.view()}
                </div>
            </td>
        </tr>
    }
}
