//! `/admin/products/{add,edit,export}` and `/admin/import`.

use super::import::{SheetMode, run_import};
use super::{Build, Check, Report, ReportView, SheetWizard, read_xlsx, save_text, stamp};
use crate::components::PageHeader;
use crate::data::load_cost_rows;
use crate::db::{Q, me};
use crate::kit::dstr;
use futures::FutureExt;
use lattice_core::api::ApiError;
use lattice_core::costing::{ProductCostLine, StageOverheads};
use lattice_core::csv::{parse_csv, to_csv};
use lattice_core::import::{Grid, ImportInput};
use lattice_core::lenient;
use lattice_core::records::ProductWithCategory;
use lattice_core::sheets::product::{LINES_SHEET, SheetProduct, product_sheets};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn admin_gate() -> Option<AnyView> {
    (!me().role.is_admin()).then(|| {
        view! { <div class="card p-6 text-sm text-(--color-muted)">"Only an admin can use this."</div> }.into_any()
    })
}

#[derive(Deserialize)]
struct CostPart {
    #[serde(default, deserialize_with = "lenient::decimal_map")]
    stage_overheads: StageOverheads,
    #[serde(default)]
    product_cost_lines: Vec<ProductCostLine>,
}

#[derive(Deserialize)]
struct ForSheet {
    #[serde(flatten)]
    p: ProductWithCategory,
    #[serde(default)]
    product_cost_sheets: Vec<CostPart>,
}

/// Every live product, paged past the server's 1000-row cap.
async fn products_for_sheet() -> Result<Vec<SheetProduct>, ApiError> {
    let mut all = Vec::new();
    for from in (0..).step_by(1000) {
        let page: Vec<ForSheet> = Q::from("products")
            .select("*,categories(name,counts_as_item),product_cost_sheets(stage_overheads,product_cost_lines(*))")
            .null("deleted_at")
            .order("code")
            .offset(from)
            .limit(1000)
            .rows()
            .await?;
        let n = page.len();
        all.extend(page.into_iter().map(|r| {
            let sheet = r.product_cost_sheets.into_iter().next();
            let p = r.p.product;
            SheetProduct {
                code: p.code,
                name: p.name,
                category_name: r.p.categories.map(|c| c.name),
                source: p.source,
                cost_price: p.cost_price,
                default_sp: p.default_sp,
                markup_pct: p.markup_pct,
                is_active: p.is_active,
                stage_overheads: sheet.as_ref().map(|s| s.stage_overheads.clone()).unwrap_or_default(),
                lines: sheet.map(|s| s.product_cost_lines).unwrap_or_default(),
            }
        }));
        if n < 1000 {
            break;
        }
    }
    Ok(all)
}

const NEW_TIP: &str = "A new product: fill any blank row on Product Master. Its code's last part is its colour (AC/0006/NT is Natural); each colour is its own row.";
const LINES_TIPS: [&str; 2] = [
    "Cost Lines tab: one row per costing line, as in the calculator. Pick the Type, Category, Subcategory and Variety from the Rates tab; Unit, Rate and Wastage fill in.",
    "Cost Price adds up the product's lines, plus each stage's overhead % (Product Master tab).",
];
const SP_TIP: &str = "Selling Price = Cost ÷ (1 − Markup %), rounded up to ₹10. You can type over it.";

/// `/admin/products/add` and `/edit`: the Product Master through Excel.
#[component]
pub fn ProductSheetPage() -> impl IntoView {
    let params = use_params_map();
    view! {
        {move || {
            if let Some(no) = admin_gate() {
                return no;
            }
            let mode = params.with(|p| p.get("mode")).unwrap_or_default();
            let sheet_mode = match mode.as_str() {
                "add" => SheetMode::Add,
                "edit" => SheetMode::Edit,
                _ => return view! { <crate::pages::NotFound /> }.into_any(),
            };
            let add = sheet_mode == SheetMode::Add;
            let build: Build = Rc::new(move || {
                async move {
                    let products = if add { Vec::new() } else { products_for_sheet().await? };
                    let rates = load_cost_rows().await?;
                    Ok(product_sheets(&products, &rates).iter().map(|s| s.data()).collect())
                }
                .boxed_local()
            });
            let check: Check = Rc::new(move |grid, apply, tabs| {
                async move {
                    let input = ImportInput { products: Some(grid), ..Default::default() };
                    let mut report = run_import(input, apply, Some(sheet_mode), tabs.get(LINES_SHEET).cloned()).await;
                    if report.applied {
                        let added = report.added.len();
                        let updated = report.count("products").saturating_sub(added);
                        let mut parts = Vec::new();
                        if added > 0 {
                            parts.push(format!("{added} new product{} added", if added == 1 { "" } else { "s" }));
                        }
                        if updated > 0 {
                            parts.push(format!("{updated} existing product{} updated", if updated == 1 { "" } else { "s" }));
                        }
                        let parts = if parts.is_empty() { "nothing to change".to_string() } else { parts.join(", ") };
                        report.message = Some(format!("Saved: {parts}."));
                    }
                    report
                }
                .boxed_local()
            });
            let mut tips = if add {
                vec!["One row per product. Codes that already exist are skipped.", NEW_TIP, "Category must match one in Settings. Active is Yes or No."]
            } else {
                vec!["Change any cell, or add new products.", NEW_TIP, "Don't change existing Product Codes; that's how rows are matched."]
            };
            tips.extend(LINES_TIPS);
            if !add {
                tips.push("A product's Cost Lines replace its costing; one with no Cost Lines keeps it.");
            }
            tips.push(SP_TIP);
            let save_text: fn(&Report) -> String = if add {
                |r| { let n = r.count("products"); format!("Add {n} product{}", if n == 1 { "" } else { "s" }) }
            } else {
                |r| { let n = r.count("products"); format!("Save {n} product{}", if n == 1 { "" } else { "s" }) }
            };
            view! {
                <PageHeader
                    title=if add { "Bulk add products" } else { "Bulk edit products" }
                    subtitle=if add { "Fill in a template in Excel, then upload it" } else { "Download your products, change them in Excel, upload them back" }
                >""</PageHeader>
                <SheetWizard
                    tips=tips
                    download_label=if add { "Download template" } else { "Download products" }
                    file_name=if add { "new-products-template.xlsx".to_string() } else { format!("product-master-{}.xlsx", stamp()) }
                    save_label=if add { "Add products" } else { "Save changes" }
                    save_text=save_text
                    done_href="/products"
                    done_label="View products"
                    make=build
                    check=check
                />
            }
            .into_any()
        }}
    }
}

/// Product Master as CSV, in the spreadsheet's column order, so it round-trips
/// through the importer (port of `api/products/export`).
async fn products_csv() -> Result<String, ApiError> {
    let rows: Vec<ProductWithCategory> =
        Q::from("products").select("*,categories(name,counts_as_item)").null("deleted_at").order("code").rows().await?;
    let mut grid: Vec<Vec<String>> = vec![
        ["Product Code", "Category", "Product Name", "Source", "Cost Price", "Target Margin %", "Default SP", "Active"]
            .map(String::from)
            .to_vec(),
    ];
    grid.extend(rows.into_iter().map(|r| {
        let p = r.product;
        vec![
            p.code,
            r.categories.map(|c| c.name).unwrap_or_default(),
            p.name,
            p.source.unwrap_or_default(),
            dstr(p.cost_price),
            dstr(p.target_margin),
            dstr(p.default_sp),
            if p.is_active { "Yes" } else { "No" }.into(),
        ]
    }));
    Ok(to_csv(&grid))
}

/// `/admin/products/export`: asks where, then saves the CSV.
#[component]
pub fn ProductsExport() -> impl IntoView {
    let status = RwSignal::new(None::<Result<bool, String>>);
    let busy = RwSignal::new(false);
    let run = move || {
        busy.set(true);
        spawn_local(async move {
            let res = match products_csv().await {
                Ok(csv) => save_text(&format!("product-master-{}.csv", stamp()), csv).await,
                Err(e) => Err(e),
            };
            busy.set(false);
            status.set(Some(res.map_err(|e| e.to_string())));
        });
    };
    run();
    view! {
        <PageHeader title="Download products" subtitle="All live products as a CSV file, in the Product Master's column order">""</PageHeader>
        <div class="card flex flex-wrap items-center gap-3 p-4">
            <button type="button" class="btn-primary" disabled=move || busy.get() on:click=move |_| run()>
                {move || if busy.get() { "Preparing…" } else { "Download CSV" }}
            </button>
            {move || match status.get() {
                Some(Ok(true)) => view! { <span class="text-sm text-emerald-700">"Saved."</span> }.into_any(),
                Some(Ok(false)) => view! { <span class="text-sm text-(--color-muted)">"Cancelled."</span> }.into_any(),
                Some(Err(e)) => view! { <span role="alert" class="text-sm text-red-700">{e}</span> }.into_any(),
                None => ().into_any(),
            }}
        </div>
    }
}

/// The six tabs, in load order; `label` is also the tab name in the workbook.
const SHEETS: [(&str, &str, &str); 6] = [
    ("settings", "Settings", "Categories and the counts-as-item flag"),
    ("products", "Product Master", "Codes, costs, margins, selling prices"),
    ("hamperSummary", "Hamper Summary", "One row per saved hamper"),
    ("hamperDetails", "Hamper Details", "The products inside each hamper"),
    ("quotationRegister", "Quotation Register", "One row per quotation or PI"),
    ("quoteDetails", "Quote Details", "The hampers on each quotation"),
];

fn input_of(grids: &[(usize, Grid)]) -> ImportInput {
    let get = |i: usize| grids.iter().find(|(k, _)| *k == i).map(|(_, g)| g.clone());
    ImportInput {
        settings: get(0),
        products: get(1),
        hamper_summary: get(2),
        hamper_details: get(3),
        quotation_register: get(4),
        quote_details: get(5),
    }
}

async fn file_text(file: &web_sys::File) -> Result<String, ApiError> {
    JsFuture::from(file.text())
        .await
        .ok()
        .and_then(|v| v.as_string())
        .ok_or_else(|| ApiError::Invalid(format!("Could not read {}. Export it again as CSV.", file.name())))
}

fn files_of(e: &leptos::ev::Event) -> Vec<web_sys::File> {
    let list = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()).and_then(|i| i.files());
    list.map(|l| (0..l.length()).filter_map(|i| l.get(i)).collect()).unwrap_or_default()
}

/// `/admin/import`: the one-time move from the old six-tab spreadsheet.
#[component]
pub fn ImportWorkbook() -> impl IntoView {
    if let Some(no) = admin_gate() {
        return no;
    }
    let grids = RwSignal::new(Vec::<(usize, Grid)>::new());
    let names = RwSignal::new(Vec::<(usize, String)>::new());
    let parse_error = RwSignal::new(None::<String>);
    let report = RwSignal::new(None::<Report>);
    let confirmed = RwSignal::new(false);
    let pending = RwSignal::new(false);
    let set_tab = move |i: usize, grid: Option<Grid>, name: String| {
        grids.update(|g| {
            g.retain(|(k, _)| *k != i);
            if let Some(grid) = &grid {
                g.push((i, grid.clone()));
            }
        });
        names.update(|n| {
            n.retain(|(k, _)| *k != i);
            if let Some(grid) = &grid {
                n.push((i, format!("{name} · {} rows", grid.len().saturating_sub(1))));
            }
        });
    };
    let reset = move || {
        report.set(None);
        confirmed.set(false);
    };
    let pick_workbook = move |file: Option<web_sys::File>| {
        reset();
        let Some(file) = file else { return };
        spawn_local(async move {
            match read_xlsx(&file).await {
                Ok(tabs) => {
                    let mut missing = Vec::new();
                    for (i, (_, label, _)) in SHEETS.iter().enumerate() {
                        match tabs.iter().find(|(n, _)| n == label) {
                            // As SheetJS read them (blankrows: false): no empty rows.
                            Some((_, g)) => set_tab(
                                i,
                                Some(g.iter().filter(|r| r.iter().any(|c| !c.is_empty())).cloned().collect()),
                                (*label).into(),
                            ),
                            None => missing.push(*label),
                        }
                    }
                    parse_error.set(
                        (!missing.is_empty())
                            .then(|| format!("{} has no tab named: {}", file.name(), missing.join(", "))),
                    );
                }
                Err(_) => {
                    parse_error.set(Some(format!("Could not read {}. Check it is a .xlsx workbook.", file.name())))
                }
            }
        });
    };
    let pick_csv = move |i: usize, file: Option<web_sys::File>| {
        reset();
        let Some(file) = file else {
            return set_tab(i, None, String::new());
        };
        spawn_local(async move {
            match file_text(&file).await {
                Ok(text) => {
                    set_tab(i, Some(parse_csv(&text)), file.name());
                    parse_error.set(None);
                }
                Err(e) => parse_error.set(Some(e.to_string())),
            }
        });
    };
    // Several CSVs at once, matched to tabs by file name ("Hamper Details.csv").
    let pick_many = move |files: Vec<web_sys::File>| {
        let norm = |v: &str| v.to_lowercase().chars().filter(char::is_ascii_lowercase).collect::<String>();
        let mut unmatched = Vec::new();
        for file in files {
            let name = norm(file.name().trim_end_matches(".csv").trim_end_matches(".CSV"));
            match SHEETS.iter().position(|(key, label, _)| name.contains(&norm(label)) || name.contains(&norm(key))) {
                Some(i) => pick_csv(i, Some(file)),
                None => unmatched.push(file.name()),
            }
        }
        if !unmatched.is_empty() {
            parse_error.set(Some(format!(
                "Couldn't tell which tab these are: {}. Name each file after its tab, or use the pickers below.",
                unmatched.join(", ")
            )));
        }
    };
    let run = move |apply: bool| {
        pending.set(true);
        let input = input_of(&grids.get_untracked());
        spawn_local(async move {
            let r = run_import(input, apply, None, None).await;
            pending.set(false);
            report.set(Some(r));
            if apply {
                confirmed.set(false);
            }
        });
    };
    let any = move || grids.with(|g| !g.is_empty());

    view! {
        <PageHeader title="Import old workbook" subtitle="Brings across products, saved hampers and past quotations, keeping their existing codes">""</PageHeader>
        <div class="space-y-4">
            <section class="card p-4">
                <h2 class="text-sm font-semibold">"1. Load the spreadsheet"</h2>
                <p class="mt-1 text-sm text-(--color-muted)">
                    "Easiest: File → Download → Microsoft Excel (.xlsx) once, and drop the whole workbook here. Columns are read by position, so leave the layout exactly as it is — renamed headers are fine, reordered columns are not."
                </p>
                <div class="mt-3 rounded-md border border-(--color-line) p-3">
                    <label class="label" for="workbook">"Whole workbook (.xlsx)"</label>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Splits into the six tabs below by name"</p>
                    <input id="workbook" type="file" accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" class="mt-2 w-full text-sm"
                        on:change=move |e| pick_workbook(files_of(&e).into_iter().next()) />
                </div>
                <div class="mt-4 rounded-md border border-(--color-line) p-3">
                    <label class="label" for="csv-many">"Several CSV files at once"</label>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Select them together; each is matched to its tab by file name, e.g. “Product Master.csv”"</p>
                    <input id="csv-many" type="file" accept=".csv,text/csv" multiple=true class="mt-2 w-full text-sm" on:change=move |e| pick_many(files_of(&e)) />
                </div>
                <p class="mt-4 text-sm text-(--color-muted)">"Or load tabs one at a time, as CSV. These override anything the workbook filled in."</p>
                <div class="mt-3 grid gap-3 sm:grid-cols-2">
                    {SHEETS.iter().enumerate().map(|(i, (key, label, hint))| view! {
                        <div class="rounded-md border border-(--color-line) p-3">
                            <label class="label" for=*key>{*label}</label>
                            <p class="mt-0.5 text-xs text-(--color-muted)">{*hint}</p>
                            <input id=*key type="file" accept=".csv,text/csv" class="mt-2 w-full text-sm" on:change=move |e| pick_csv(i, files_of(&e).into_iter().next()) />
                            {move || names.with(|n| n.iter().find(|(k, _)| *k == i).map(|(_, s)| view! { <p class="mt-1 text-xs text-green-800">{s.clone()}</p> }))}
                        </div>
                    }).collect_view()}
                </div>
                {move || parse_error.get().map(|e| view! { <p role="alert" class="mt-3 text-sm text-red-700">{e}</p> })}
            </section>
            <section class="card p-4">
                <h2 class="text-sm font-semibold">"2. Check what will be imported"</h2>
                <p class="mt-1 text-sm text-(--color-muted)">"A dry run reads the files and reports what it found. Nothing is written yet."</p>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    <button type="button" class="btn-secondary" disabled=move || !any() || pending.get() on:click=move |_| run(false)>
                        {move || if pending.get() { "Checking…" } else { "Dry run" }}
                    </button>
                </div>
                {move || report.get().map(|r| view! { <ReportView report=r /> })}
            </section>
            <section class="card p-4">
                <h2 class="text-sm font-semibold">"3. Import"</h2>
                <p class="mt-1 text-sm text-(--color-muted)">
                    "Records are matched on product code, hamper code and document number, so running this twice updates rather than duplicates. Existing hamper and quotation lines are replaced by what is in the files."
                </p>
                <label class="mt-3 flex items-start gap-2 text-sm">
                    <input type="checkbox" class="mt-0.5" prop:checked=confirmed on:change=move |e| confirmed.set(event_target_checked(&e)) />
                    "I have run the dry run above and the numbers look right."
                </label>
                <button type="button" class="btn-primary mt-3" disabled=move || !any() || !confirmed.get() || pending.get() on:click=move |_| run(true)>
                    {move || if pending.get() { "Importing…" } else { "Import into the database" }}
                </button>
            </section>
        </div>
    }
    .into_any()
}
