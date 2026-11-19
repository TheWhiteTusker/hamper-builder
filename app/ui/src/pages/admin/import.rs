//! The importer behind the workbook import and the Product Master sheet
//! (ports of `admin/import/actions.ts`, `save.ts`, `save-costing.ts`). Dry run
//! unless `apply`; everything is matched on its natural key, so a re-run
//! updates rather than duplicates.

use super::Report;
use crate::data::{load_cost_rows, load_settings};
use crate::db::{self, Q, insert, insert_returning};
use crate::kit::{dstr, product_href};
use crate::pages::calculator::line_row;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::colors::color_code;
use lattice_core::cost_tree::build_cost_tree;
use lattice_core::costing::{ProductCostLine, StageOverheads};
use lattice_core::import::{Grid, ImportInput, ImportResult, ProductRow, body, build_import, cell};
use lattice_core::js::{self, dec, f};
use lattice_core::lenient;
use lattice_core::mrp::mrp_limit;
use lattice_core::product_code::parse_product_code;
use lattice_core::settings::merge_list;
use lattice_core::sheets::product::{CostingPlan, ExistingCosting, SheetCost, plan_sheet_costing, read_cost_lines, read_overheads};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const CHUNK: usize = 500;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SheetMode {
    Add,
    Edit,
}

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

async fn in_chunks(table: &str, rows: &[Value], on_conflict: Option<&str>) -> Result<(), ApiError> {
    for batch in rows.chunks(CHUNK) {
        insert(table, &batch, on_conflict, None).await?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct CodeRow {
    id: String,
    code: String,
    deleted_at: Option<String>,
}

/// Insert-or-update on `code`. Not an upsert on code: since the Bin, code is
/// unique only among live rows (a partial index ON CONFLICT can't target). A
/// code matching a binned row brings it back. Returns the codes that were new.
async fn save_by_code(table: &str, rows: Vec<Map<String, Value>>) -> Result<Vec<String>, ApiError> {
    let mut added = Vec::new();
    for batch in rows.chunks(200) {
        let codes: Vec<&str> = batch.iter().filter_map(|r| r.get("code").and_then(Value::as_str)).collect();
        let found: Vec<CodeRow> = Q::from(table).select("id,code,deleted_at").in_list("code", &codes).rows().await?;
        // A live row wins over binned copies of the same code.
        let mut id_by: HashMap<String, String> = HashMap::new();
        for r in found {
            if !id_by.contains_key(&r.code) || r.deleted_at.is_none() {
                id_by.insert(r.code, r.id);
            }
        }
        let (mut updates, mut inserts) = (Vec::new(), Vec::new());
        for r in batch {
            let code = r.get("code").and_then(Value::as_str).unwrap_or_default().to_string();
            match id_by.get(&code) {
                Some(id) => {
                    let mut row = r.clone();
                    row.insert("id".into(), json!(id));
                    row.insert("deleted_at".into(), Value::Null);
                    updates.push(Value::Object(row));
                }
                None => {
                    inserts.push(Value::Object(r.clone()));
                    added.push(code);
                }
            }
        }
        if !updates.is_empty() {
            insert(table, &updates, Some("id"), None).await?;
        }
        if !inserts.is_empty() {
            insert(table, &inserts, None, None).await?;
        }
    }
    Ok(added)
}

/// The stored margin a price gives, as the cost calculator works it out.
fn margin_of(sp: Decimal, cost: Decimal) -> Decimal {
    let (sp, cost) = (f(sp), f(cost));
    dec(if sp > 0.0 { js::round((sp - cost) / sp * 10000.0) / 10000.0 } else { 0.0 })
}

fn list(xs: &[String]) -> String {
    format!("{}{}", xs.iter().take(8).cloned().collect::<Vec<_>>().join(", "), if xs.len() > 8 { "…" } else { "" })
}

/// Rows from the Product Master sheet. Its column F is Markup % (read into
/// target_margin), and its SP may be typed over, so the stored margin is
/// worked out from the price, as the calculator does. "add" drops codes that
/// exist (Bin included). A new product takes its colour from its code; an
/// error when that isn't a colour in Settings.
async fn prepare_sheet_products(result: &mut ImportResult, mode: SheetMode) -> Result<Option<String>, ApiError> {
    for p in &mut result.products {
        p.markup_pct = Some(dec(js::round(f(p.target_margin) * 10000.0) / 100.0));
        p.target_margin = margin_of(p.default_sp, p.cost_price);
    }
    if result.products.is_empty() {
        return Ok(None);
    }
    #[derive(Deserialize)]
    struct Code {
        code: String,
    }
    let codes: Vec<String> = result.products.iter().map(|p| p.code.clone()).collect();
    let mut existing: HashSet<String> = HashSet::new();
    for batch in codes.chunks(200) {
        for r in Q::from("products").select("code").in_list("code", batch).rows::<Code>().await? {
            existing.insert(r.code);
        }
    }
    if mode == SheetMode::Add && !existing.is_empty() {
        result.products.retain(|p| !existing.contains(&p.code));
        result.counts.products = result.products.len();
        let mut known: Vec<String> = existing.iter().cloned().collect();
        known.sort();
        result.warnings.push(format!(
            "{} code(s) already exist and will be skipped. Use Bulk edit to change them: {}",
            existing.len(),
            list(&known)
        ));
    }
    let colors = load_settings().await?.product_colors;
    let by_code: Vec<(String, String)> = colors.iter().map(|c| (color_code(c), c.clone())).collect();
    let mut uncoloured = Vec::new();
    for p in &mut result.products {
        if existing.contains(&p.code) {
            continue;
        }
        let parsed = parse_product_code(&p.code);
        match by_code.iter().find(|(code, _)| parsed.is_valid && *code == parsed.color_code) {
            Some((_, name)) => p.colors = Some(vec![name.clone()]),
            None => uncoloured.push(p.code.clone()),
        }
    }
    if uncoloured.is_empty() {
        return Ok(None);
    }
    let known = by_code.iter().map(|(c, n)| format!("{c} {n}")).collect::<Vec<_>>().join(", ");
    Ok(Some(format!(
        "New product codes must look like AC/0006/NT, ending in a colour code from Settings ({known}). Check: {}",
        list(&uncoloured)
    )))
}

#[derive(Deserialize)]
struct SheetRow {
    id: String,
    #[serde(default, deserialize_with = "lenient::decimal_map")]
    stage_overheads: StageOverheads,
    #[serde(default)]
    product_cost_lines: Vec<ProductCostLine>,
}

#[derive(Deserialize)]
struct CostedRow {
    code: String,
    deleted_at: Option<String>,
    #[serde(default)]
    product_cost_sheets: Vec<SheetRow>,
}

/// One plan per product whose costing the Cost Lines tab changes; sets its
/// cost_price and margin. Planned on the dry run, so warnings show first.
async fn plan_costings(result: &mut ImportResult, products: Option<&Grid>, lines: Option<&Grid>) -> Result<(Vec<CostingPlan>, Vec<String>), ApiError> {
    let mut errors = Vec::new();
    if body(lines).is_empty() {
        return Ok((Vec::new(), errors));
    }
    let stages = build_cost_tree(&load_cost_rows().await?);
    let sheet_lines = read_cost_lines(lines, &stages, &mut result.warnings, &mut errors);
    let overheads: HashMap<String, StageOverheads> =
        body(products).into_iter().map(|row| (cell(Some(row), 0), read_overheads(row))).collect();
    let strays: Vec<String> = sheet_lines.keys().filter(|c| !overheads.contains_key(*c)).cloned().collect();
    if !strays.is_empty() {
        result.warnings.push(format!(
            "Cost Lines for codes not on the Product Master tab are ignored: {}",
            strays.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    let costed: Vec<String> = result.products.iter().filter(|p| sheet_lines.contains_key(&p.code)).map(|p| p.code.clone()).collect();
    if costed.is_empty() || !errors.is_empty() {
        return Ok((Vec::new(), errors));
    }
    // A live product wins over binned copies of its code.
    let mut rows: HashMap<String, CostedRow> = HashMap::new();
    for batch in costed.chunks(200) {
        let got: Vec<CostedRow> = Q::from("products")
            .select("code,deleted_at,product_cost_sheets(id,stage_overheads,product_cost_lines(*))")
            .in_list("code", batch)
            .rows()
            .await?;
        for r in got {
            if !rows.contains_key(&r.code) || r.deleted_at.is_none() {
                rows.insert(r.code.clone(), r);
            }
        }
    }
    let gst = load_settings().await?.gst_rate;
    let mut plans: BTreeMap<String, CostingPlan> = BTreeMap::new();
    let mut warnings = std::mem::take(&mut result.warnings);
    for p in result.products.iter_mut().filter(|p| sheet_lines.contains_key(&p.code)) {
        let existing = rows.get(&p.code).and_then(|r| r.product_cost_sheets.first()).map(|s| ExistingCosting {
            sheet_id: s.id.clone(),
            overheads: s.stage_overheads.clone(),
            lines: s.product_cost_lines.clone(),
        });
        let empty = StageOverheads::new();
        let sheet_cost = SheetCost { code: &p.code, cost_price: p.cost_price, markup_pct: p.markup_pct.unwrap_or_default() };
        let Some(plan) = plan_sheet_costing(
            &sheet_cost,
            sheet_lines.get(&p.code).map(Vec::as_slice).unwrap_or_default(),
            overheads.get(&p.code).unwrap_or(&empty),
            existing.as_ref(),
            &mut warnings,
        ) else {
            continue;
        };
        p.cost_price = plan.totals.total_cost;
        p.target_margin = margin_of(p.default_sp, p.cost_price);
        if let Some(limit) = mrp_limit(&plan.lines, Some(&plan.overheads), plan.totals.total_cost, gst)
            && p.default_sp > limit
        {
            warnings.push(format!("{}: SP ₹{} is above its MRP limit of ₹{}.", p.code, dstr(p.default_sp), dstr(limit)));
        }
        plans.insert(p.code.clone(), plan);
    }
    result.warnings = warnings;
    Ok((plans.into_values().collect(), errors))
}

/// Writes the plans once their products are saved (new ones included).
async fn save_sheet_costing(plans: &[CostingPlan], products: &[ProductRow]) -> Result<(), ApiError> {
    if plans.is_empty() {
        return Ok(());
    }
    #[derive(Deserialize)]
    struct IdCode {
        id: String,
        code: String,
    }
    let by_code: HashMap<&str, &ProductRow> = products.iter().map(|p| (p.code.as_str(), p)).collect();
    let mut product_id: HashMap<String, String> = HashMap::new();
    for batch in plans.chunks(200) {
        let codes: Vec<&str> = batch.iter().map(|p| p.code.as_str()).collect();
        for r in Q::from("products").select("id,code").in_list("code", &codes).null("deleted_at").rows::<IdCode>().await? {
            product_id.insert(r.code, r.id);
        }
    }
    let values = |plan: &CostingPlan| -> Map<String, Value> {
        let p = by_code.get(plan.code.as_str());
        let t = &plan.totals;
        obj(json!({
            "product_id": product_id.get(&plan.code),
            "product_code": plan.code,
            "product_name": p.map(|p| p.name.clone()),
            "material_total": t.material_total,
            "hardware_total": t.hardware_total,
            "finishing_total": t.finishing_total,
            "machine_total": t.machine_total,
            "total_cost": t.total_cost,
            "markup_pct": p.and_then(|p| p.markup_pct).unwrap_or_default(),
            "calculated_sp": p.map(|p| p.default_sp).unwrap_or_default(),
            "stage_overheads": plan.overheads.iter().map(|(k, v)| (k.clone(), crate::kit::json_num(f(*v)))).collect::<Map<_, _>>(),
        }))
    };
    let existing: Vec<&CostingPlan> = plans.iter().filter(|p| p.sheet_id.is_some()).collect();
    let mut sheet_id: HashMap<String, String> =
        existing.iter().filter_map(|p| p.sheet_id.clone().map(|id| (p.code.clone(), id))).collect();
    let upserts: Vec<Value> = existing
        .iter()
        .map(|p| {
            let mut v = values(p);
            v.insert("id".into(), json!(p.sheet_id));
            Value::Object(v)
        })
        .collect();
    in_chunks("product_cost_sheets", &upserts, Some("id")).await?;
    let fresh: Vec<&CostingPlan> = plans.iter().filter(|p| p.sheet_id.is_none() && product_id.contains_key(&p.code)).collect();
    #[derive(Deserialize)]
    struct NewSheet {
        id: String,
        product_code: String,
    }
    for batch in fresh.chunks(CHUNK) {
        let rows: Vec<Value> = batch.iter().map(|p| Value::Object(values(p))).collect();
        for s in insert_returning::<_, NewSheet>("product_cost_sheets", &rows).await? {
            sheet_id.insert(s.product_code, s.id);
        }
    }
    // The sheet's lines replace the saved ones.
    let replaced: Vec<String> = existing.iter().filter_map(|p| p.sheet_id.clone()).collect();
    for batch in replaced.chunks(200) {
        Q::from("product_cost_lines").in_list("sheet_id", batch).delete().await?;
    }
    let lines: Vec<Value> = plans
        .iter()
        .flat_map(|p| sheet_id.get(&p.code).map(|id| p.lines.iter().map(|l| line_row(l, id)).collect::<Vec<_>>()).unwrap_or_default())
        .collect();
    in_chunks("product_cost_lines", &lines, None).await
}

fn counts(result: &ImportResult, costings: usize) -> Vec<(&'static str, usize)> {
    let c = &result.counts;
    let mut out = vec![
        ("categories", c.categories),
        ("products", c.products),
        ("hampers", c.hampers),
        ("hamperItems", c.hamper_items),
        ("quotes", c.quotes),
        ("quoteItems", c.quote_items),
    ];
    if costings > 0 {
        out.push(("costings", costings));
    }
    out
}

#[derive(Deserialize)]
struct IdCode {
    id: String,
    code: String,
}

/// Port of `runImport`. With `sheet`, rows come from the Product Master sheet
/// and `cost_lines` is that workbook's Cost Lines tab.
pub async fn run_import(input: ImportInput, apply: bool, sheet: Option<SheetMode>, cost_lines: Option<Grid>) -> Report {
    match import(input, apply, sheet, cost_lines).await {
        Ok(r) => r,
        Err(e) => Report::failed(e),
    }
}

async fn import(input: ImportInput, apply: bool, sheet: Option<SheetMode>, cost_lines: Option<Grid>) -> Result<Report, ApiError> {
    let mut result = build_import(&input);
    let mut costings = Vec::new();
    if let Some(mode) = sheet {
        if let Some(invalid) = prepare_sheet_products(&mut result, mode).await? {
            return Ok(Report::failed(invalid));
        }
        let (plans, errors) = plan_costings(&mut result, input.products.as_ref(), cost_lines.as_ref()).await?;
        if !errors.is_empty() {
            return Ok(Report::failed(errors.iter().take(8).cloned().collect::<Vec<_>>().join(" ")));
        }
        costings = plans;
    }
    if !apply {
        return Ok(Report { ok: true, counts: counts(&result, costings.len()), warnings: result.warnings, ..Default::default() });
    }

    // ---- categories
    let to_rows = |v: Value| v.as_array().cloned().unwrap_or_default();
    if !result.categories.is_empty() {
        in_chunks("categories", &to_rows(json!(result.categories)), Some("name")).await?;
    }
    #[derive(Deserialize)]
    struct Named {
        id: String,
        name: String,
    }
    let category_id: HashMap<String, String> =
        Q::from("categories").select("id,name").rows::<Named>().await?.into_iter().map(|c| (c.name.trim().to_lowercase(), c.id)).collect();

    // ---- products
    let mut added_codes = Vec::new();
    if !result.products.is_empty() {
        let rows: Vec<Map<String, Value>> = result
            .products
            .iter()
            .map(|p| {
                let mut m = obj(json!(p));
                let cat = m.remove("category_name").and_then(|v| v.as_str().map(str::to_string));
                m.insert("category_id".into(), json!(cat.and_then(|c| category_id.get(&c.trim().to_lowercase()).cloned())));
                m
            })
            .collect();
        added_codes = save_by_code("products", rows).await?;
        save_sheet_costing(&costings, &result.products).await?;
    }

    // ---- hampers
    if !result.hampers.is_empty() {
        save_by_code("hampers", to_rows(json!(result.hampers)).into_iter().map(obj).collect()).await?;
    }
    let hamper_id: HashMap<String, String> = Q::from("hampers").select("id,code").rows::<IdCode>().await?.into_iter().map(|h| (h.code, h.id)).collect();
    let product_id: HashMap<String, String> = Q::from("products").select("id,code").rows::<IdCode>().await?.into_iter().map(|p| (p.code, p.id)).collect();
    if !result.hamper_items.is_empty() {
        // Replace the lines of every hamper imported, so a re-run doesn't append a second copy.
        let touched: Vec<String> = result.hamper_items.iter().map(|i| i.hamper_code.clone()).collect::<HashSet<_>>().into_iter().filter_map(|c| hamper_id.get(&c).cloned()).collect();
        for batch in touched.chunks(CHUNK) {
            Q::from("hamper_items").in_list("hamper_id", batch).delete().await?;
        }
        let rows: Vec<Value> = result
            .hamper_items
            .iter()
            .filter(|i| hamper_id.contains_key(&i.hamper_code))
            .map(|i| {
                let mut m = obj(json!(i));
                m.remove("hamper_code");
                m.insert("hamper_id".into(), json!(hamper_id.get(&i.hamper_code)));
                m.insert("product_id".into(), json!(i.product_code.as_ref().and_then(|c| product_id.get(c))));
                Value::Object(m)
            })
            .collect();
        in_chunks("hamper_items", &rows, None).await?;
    }

    // ---- quotes
    if !result.quotes.is_empty() {
        in_chunks("quotes", &to_rows(json!(result.quotes)), Some("doc_no")).await?;
    }
    #[derive(Deserialize)]
    struct Doc {
        id: String,
        doc_no: String,
    }
    let quote_id: HashMap<String, String> = Q::from("quotes").select("id,doc_no").rows::<Doc>().await?.into_iter().map(|q| (q.doc_no, q.id)).collect();
    if !result.quote_items.is_empty() {
        let touched: Vec<String> = result.quote_items.iter().map(|i| i.doc_no.clone()).collect::<HashSet<_>>().into_iter().filter_map(|d| quote_id.get(&d).cloned()).collect();
        for batch in touched.chunks(CHUNK) {
            Q::from("quote_items").in_list("quote_id", batch).delete().await?;
        }
        let rows: Vec<Value> = result
            .quote_items
            .iter()
            .filter(|i| quote_id.contains_key(&i.doc_no))
            .map(|i| {
                let mut m = obj(json!(i));
                m.remove("doc_no");
                m.insert("quote_id".into(), json!(quote_id.get(&i.doc_no)));
                m.insert("hamper_id".into(), json!(i.hamper_code.as_ref().and_then(|c| hamper_id.get(c))));
                Value::Object(m)
            })
            .collect();
        in_chunks("quote_items", &rows, None).await?;
    }

    // ---- pick-lists: real strings from the spreadsheet beat the guessed defaults.
    let s = load_settings().await?;
    let r = &result;
    let mut updates = vec![
        ("collections", json!(merge_list(&s.collections, &r.hampers.iter().map(|h| h.collection.clone()).collect::<Vec<_>>()))),
        ("sources", json!(merge_list(&s.sources, &r.products.iter().map(|p| p.source.clone()).collect::<Vec<_>>()))),
        ("hamper_statuses", json!(merge_list(&s.hamper_statuses, &r.hampers.iter().map(|h| Some(h.status.clone())).collect::<Vec<_>>()))),
        ("quote_statuses", json!(merge_list(&s.quote_statuses, &r.quotes.iter().map(|q| Some(q.status.clone())).collect::<Vec<_>>()))),
        ("quote_structures", json!(merge_list(&s.quote_structures, &r.quotes.iter().map(|q| Some(q.quote_structure.clone())).collect::<Vec<_>>()))),
        ("validity_options", json!(merge_list(&s.validity_options, &r.quotes.iter().map(|q| q.validity.clone()).collect::<Vec<_>>()))),
        (
            "detail_modes",
            json!(merge_list(
                &s.detail_modes,
                &std::iter::once(r.default_detail_mode.clone()).chain(r.quote_items.iter().map(|i| i.detail_mode.clone())).collect::<Vec<_>>()
            )),
        ),
        ("packaging_treatments", json!(merge_list(&s.packaging_treatments, &r.quote_items.iter().map(|i| i.packaging_treatment.clone()).collect::<Vec<_>>()))),
    ];
    if let Some(d) = &r.default_detail_mode {
        updates.push(("default_detail_mode", json!(d)));
    }
    let rows: Vec<Value> = updates.into_iter().map(|(k, v)| json!({ "key": k, "value": v })).collect();
    insert("app_settings", &rows, Some("key"), None).await?;
    // Numbering continues from the highest imported H### / LLQT-### / LLPI-###.
    db::rpc::<_, Value>("sync_doc_counters", &json!({})).await?;
    db::cache_drop(&["products", "catalog", "hampers", "quotes", "categories", "settings", "dashboard", "quote_options", "cost_rows"]).await;

    let added = result
        .products
        .iter()
        .filter(|p| added_codes.contains(&p.code))
        .map(|p| (format!("{} · {}", p.code, p.name), Some(product_href(&p.code))))
        .collect();
    Ok(Report { ok: true, applied: true, counts: counts(&result, costings.len()), warnings: result.warnings, added, ..Default::default() })
}
