//! Admin tools (ports of `(app)/admin/*` and the rates Excel round trip):
//! the sheet wizards, the one-time workbook import, refresh prices, and the
//! products CSV.

mod import;
mod pages;
mod rates;
mod refresh;

pub use pages::{ImportWorkbook, ProductSheetPage, ProductsExport};
pub use rates::RateSheetPage;
pub use refresh::RefreshPrices;

use crate::tauri;
use futures::future::LocalBoxFuture;
use lattice_core::api::ApiError;
use lattice_core::import::Grid;
use lattice_core::sheets::SheetData;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use serde_json::json;
use std::collections::BTreeMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Every tab of an upload, by name.
pub type Tabs = BTreeMap<String, Grid>;

/// What a dry run (or the real import) found (port of `ImportReport`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub ok: bool,
    pub applied: bool,
    /// In display order: (key, count).
    pub counts: Vec<(&'static str, usize)>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    /// Shown once applied, in place of the full import's wording.
    pub message: Option<String>,
    /// What was newly created: (label, link).
    pub added: Vec<(String, Option<String>)>,
}

impl Report {
    pub fn failed(e: impl ToString) -> Self {
        Self { error: Some(e.to_string()), ..Default::default() }
    }

    pub fn count(&self, key: &str) -> usize {
        self.counts.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n)
    }
}

/// YYYY-MM-DD, for file names.
pub fn stamp() -> String {
    js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default().chars().take(10).collect()
}

/// Asks where, then writes the workbook. Ok(false) if the dialog was cancelled.
pub async fn save_xlsx(file_name: &str, sheets: Vec<SheetData>) -> Result<bool, ApiError> {
    let path: Option<String> = tauri::invoke("save_xlsx", &json!({ "file_name": file_name, "sheets": sheets })).await?;
    Ok(path.is_some())
}

pub async fn save_text(file_name: &str, content: String) -> Result<bool, ApiError> {
    let path: Option<String> =
        tauri::invoke("save_text", &json!({ "file_name": file_name, "content": content })).await?;
    Ok(path.is_some())
}

pub async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, ApiError> {
    let buf = JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| ApiError::Invalid(format!("Couldn't read {}.", file.name())))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Every tab of an `.xlsx`, read natively, in the workbook's order.
pub async fn read_xlsx(file: &web_sys::File) -> Result<Vec<(String, Grid)>, ApiError> {
    let bytes = file_bytes(file).await?;
    tauri::invoke_raw("read_xlsx", &bytes, &[]).await
}

pub type Build = Rc<dyn Fn() -> LocalBoxFuture<'static, Result<Vec<SheetData>, ApiError>>>;
pub type Check = Rc<dyn Fn(Grid, bool, Tabs) -> LocalBoxFuture<'static, Report>>;

fn label_for(key: &str) -> &str {
    match key {
        "categories" => "Categories",
        "products" => "Products",
        "costings" => "Costings from Cost Lines",
        "hampers" => "Hampers",
        "hamperItems" => "Hamper lines",
        "quotes" => "Quotations",
        "quoteItems" => "Quotation lines",
        "newCategories" => "New categories",
        "newSubcategories" => "New subcategories",
        "newVarieties" => "New varieties",
        "updatedVarieties" => "Varieties changed",
        other => other,
    }
}

/// Counts, warnings, and "done" (port of `import-report.tsx`).
#[component]
pub fn ReportView(report: Report) -> impl IntoView {
    let Report { applied, counts, warnings, error, message, added, .. } = report;
    if let Some(e) = error {
        return view! { <p role="alert" class="mt-4 text-sm text-red-700">{e}</p> }.into_any();
    }
    let n_added = added.len();
    view! {
        <div class="mt-4">
            <div class="grid gap-2 sm:grid-cols-3">
                {counts.into_iter().map(|(k, n)| view! {
                    <div class="rounded-md border border-(--color-line) px-3 py-2">
                        <div class="label">{label_for(k).to_string()}</div>
                        <div class="text-lg font-semibold tabular-nums">{n}</div>
                    </div>
                }).collect_view()}
            </div>
            {(!warnings.is_empty()).then(|| view! {
                <ul class="mt-3 space-y-1.5 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
                    {warnings.into_iter().map(|w| view! { <li>{w}</li> }).collect_view()}
                </ul>
            })}
            {applied.then(|| view! {
                <p class="mt-3 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">
                    {message.unwrap_or_else(|| "Import complete. Document numbering now continues from the highest imported code.".into())}
                </p>
            })}
            {(applied && n_added > 0).then(|| view! {
                <div class="mt-3 rounded-md border border-(--color-line) p-3">
                    <div class="label">{format!("Newly added ({n_added})")}</div>
                    <ul class="mt-1.5 max-h-72 space-y-1 overflow-y-auto text-sm">
                        {added.into_iter().map(|(label, href)| match href {
                            Some(h) => view! { <li><A href=h attr:class="text-(--color-brand) underline">{label}</A></li> }.into_any(),
                            None => view! { <li>{label}</li> }.into_any(),
                        }).collect_view()}
                    </ul>
                </div>
            })}
        </div>
    }
    .into_any()
}

#[component]
fn Step(n: u8, title: &'static str, #[prop(into)] done: Signal<bool>, children: Children) -> impl IntoView {
    view! {
        <section class="card p-4">
            <h2 class="flex items-center gap-2 text-sm font-semibold">
                <span class=move || format!("flex h-6 w-6 items-center justify-center rounded-full text-xs font-bold {}",
                    if done.get() { "bg-emerald-600 text-white" } else { "bg-(--color-brand-soft) text-(--color-brand-dark)" })>
                    {move || if done.get() { "✓".to_string() } else { n.to_string() }}
                </span>
                {title}
            </h2>
            <div class="mt-3">{children()}</div>
        </section>
    }
}

/// Download → fill in Excel → upload → save (port of `sheet-wizard.tsx`). The
/// upload is checked first; nothing is written until Save.
#[component]
pub fn SheetWizard(
    tips: Vec<&'static str>,
    download_label: &'static str,
    file_name: String,
    save_label: &'static str,
    /// The Save button's wording once the check has counted what will change.
    save_text: fn(&Report) -> String,
    done_href: &'static str,
    done_label: &'static str,
    make: Build,
    check: Check,
) -> impl IntoView {
    let build = StoredValue::new_local(make);
    let check = StoredValue::new_local(check);
    let file_name = StoredValue::new(file_name);
    let downloaded = RwSignal::new(false);
    let downloading = RwSignal::new(false);
    let upload = StoredValue::new_local(None::<(Grid, Tabs)>);
    let loaded = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let report = RwSignal::new(None::<Report>);
    let pending = RwSignal::new(false);
    let dragging = RwSignal::new(false);
    let has_grid = RwSignal::new(false);

    let download = move || {
        downloading.set(true);
        error.set(None);
        let make = build.with_value(|b| b());
        spawn_local(async move {
            let res = match make.await {
                Ok(sheets) => save_xlsx(&file_name.get_value(), sheets).await,
                Err(e) => Err(e),
            };
            downloading.set(false);
            match res {
                Ok(true) => downloaded.set(true),
                Ok(false) => {}
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };
    let run = move |apply: bool| {
        let Some((grid, tabs)) = upload.with_value(Clone::clone) else { return };
        pending.set(true);
        let fut = check.with_value(|c| c(grid, apply, tabs));
        spawn_local(async move {
            let r = fut.await;
            pending.set(false);
            report.set(Some(r));
        });
    };
    let pick = move |file: Option<web_sys::File>| {
        report.set(None);
        error.set(None);
        has_grid.set(false);
        let Some(file) = file else { return };
        spawn_local(async move {
            match read_xlsx(&file).await {
                Ok(tabs) if !tabs.is_empty() => {
                    let first = tabs.first().map(|(_, g)| g.clone()).unwrap_or_default();
                    upload.set_value(Some((first, tabs.into_iter().collect())));
                    loaded.set(file.name());
                    has_grid.set(true);
                    run(false);
                }
                _ => error.set(Some(format!("Couldn't read {}. Use the .xlsx file from step 1.", file.name()))),
            }
        });
    };
    let counts_any = move || report.with(|r| r.as_ref().is_some_and(|r| r.counts.iter().any(|(_, n)| *n > 0)));
    let saved = move || report.with(|r| r.as_ref().is_some_and(|r| r.applied && r.ok));

    view! {
        <div class="space-y-3">
            <Step n=1 title="Download" done=Signal::derive(move || downloaded.get() || has_grid.get())>
                <ul class="list-disc space-y-0.5 pl-5 text-sm text-(--color-muted)">
                    {tips.into_iter().map(|t| view! { <li>{t}</li> }).collect_view()}
                    <li>"Blue headings: type here. Grey: worked out for you."</li>
                </ul>
                <button type="button" class="btn-secondary mt-3" disabled=move || downloading.get() on:click=move |_| download()>
                    {move || if downloading.get() { "Preparing…" } else { download_label }}
                </button>
            </Step>
            <Step n=2 title="Upload" done=has_grid>
                <label
                    class=move || format!("flex cursor-pointer flex-col items-center gap-1 rounded-lg border-2 border-dashed px-4 py-6 text-center text-sm transition-colors {}",
                        if dragging.get() { "border-(--color-brand) bg-(--color-brand-soft)" } else { "border-(--color-line) hover:bg-(--color-paper)" })
                    on:dragover=move |e| { e.prevent_default(); dragging.set(true); }
                    on:dragleave=move |_| dragging.set(false)
                    on:drop=move |e| {
                        e.prevent_default();
                        dragging.set(false);
                        pick(e.data_transfer().and_then(|d| d.files()).and_then(|f| f.get(0)));
                    }
                >
                    <span class="font-medium">{move || { let l = loaded.get(); if l.is_empty() { "Drop your Excel file here".to_string() } else { l } }}</span>
                    <span class="text-xs text-(--color-muted)">{move || if loaded.with(String::is_empty) { "or click to choose" } else { "Click to choose a different file" }}</span>
                    <input type="file" accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" class="sr-only"
                        on:change=move |e| {
                            let input = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
                            pick(input.as_ref().and_then(|i| i.files()).and_then(|f| f.get(0)));
                            if let Some(i) = input {
                                i.set_value("");
                            }
                        } />
                </label>
                {move || error.get().map(|e| view! { <p role="alert" class="mt-3 text-sm text-red-700">{e}</p> })}
                <Show when=move || pending.get() && report.with(Option::is_none)><p class="mt-3 text-sm text-(--color-muted)">"Checking…"</p></Show>
                {move || report.get().map(|r| view! { <ReportView report=r /> })}
            </Step>
            <Step n=3 title="Save" done=Signal::derive(saved)>
                <p class="text-sm text-(--color-muted)">"Nothing changes until you press this."</p>
                <div class="mt-3 flex flex-wrap gap-2">
                    <button type="button" class="btn-primary"
                        disabled=move || !has_grid.get() || pending.get() || !report.with(|r| r.as_ref().is_some_and(|r| r.ok && !r.applied)) || !counts_any()
                        on:click=move |_| run(true)>
                        {move || {
                            if pending.get() && report.with(Option::is_some) {
                                return "Saving…".to_string();
                            }
                            report.with(|r| match r {
                                Some(r) if r.ok && !r.applied && r.counts.iter().any(|(_, n)| *n > 0) => save_text(r),
                                _ => save_label.to_string(),
                            })
                        }}
                    </button>
                    <Show when=saved><A href=done_href attr:class="btn-secondary">{done_label}</A></Show>
                </div>
            </Step>
        </div>
    }
}
