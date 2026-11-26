//! Saving a costing: the same steps, in the same order, as the Next app's
//! `saveCostSheetAndProduct` (with `save-sheet.ts`, `bought-out-master.ts` and
//! `reprice.ts`). Several requests, as there; see the plan for the RPC that
//! would make it one transaction.

use crate::data::load_settings;
use crate::db::{self, Q};
use lattice_core::api::ApiError;
use lattice_core::costing::{ProductCostLine, StageOverheads, calculate_cost_sheet_totals, calculate_line_cost};
use lattice_core::js::{dec, f};
use lattice_core::mrp::{mrp_limit, round_sp};
use lattice_core::product_code::{codes_for_colors, parse_product_code};
use lattice_core::reprice::{MasterRate, PricedProduct, RepriceInput, SheetRef, plan_reprice};
use lattice_core::{Decimal, lenient};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct Payload {
    pub sheet_id: Option<String>,
    pub product_id: Option<String>,
    pub code: String,
    pub name: String,
    pub category_id: Option<String>,
    pub source: Option<String>,
    pub colors: Vec<String>,
    pub is_active: bool,
    pub markup_pct: Decimal,
    pub stage_overheads: StageOverheads,
    pub selling_price: Decimal,
    pub notes: Option<String>,
    pub lines: Vec<ProductCostLine>,
    /// Existing sibling colours (product ids) that take this price and costing too.
    pub apply_to: Vec<String>,
    pub user_id: String,
}

pub struct Variant {
    pub color: String,
    pub code: String,
    pub id: String,
}

pub struct Saved {
    pub product_id: String,
    pub variants: Vec<Variant>,
    pub total_cost: Decimal,
    pub selling_price: Decimal,
}

fn err(prefix: &str) -> impl Fn(ApiError) -> ApiError + '_ {
    move |e| ApiError::Server(format!("{prefix}: {e}"))
}

#[derive(Deserialize)]
struct Id {
    id: String,
}

/// Cost lines as stored: recomputed totals, coerced numbers, 1-based order.
pub fn prepare_lines(lines: &[ProductCostLine]) -> Vec<ProductCostLine> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let calc = calculate_line_cost(l);
            let sub = l.subcategory_name.clone().unwrap_or_default();
            let var = l.variety_name.clone().unwrap_or_default();
            let item = if !l.item_name.is_empty() {
                l.item_name.clone()
            } else if !sub.is_empty() && !var.is_empty() {
                format!("{sub} {var}")
            } else if !sub.is_empty() {
                sub.clone()
            } else {
                var.clone()
            };
            ProductCostLine {
                id: None,
                sheet_id: None,
                stage_code: l.stage_code.clone(),
                category_name: l.category_name.clone(),
                subcategory_name: (!sub.is_empty()).then_some(sub),
                variety_name: (!var.is_empty()).then_some(var),
                item_name: item,
                cost_item_id: l.cost_item_id.clone().filter(|s| !s.is_empty()),
                cost_variety_id: l.cost_variety_id.clone().filter(|s| !s.is_empty()),
                length: l.length,
                breadth: l.breadth,
                dimension_unit: Some(
                    l.dimension_unit.clone().filter(|u| !u.is_empty()).unwrap_or_else(|| "inch".into()),
                ),
                unit: if l.unit.is_empty() { "sq ft".into() } else { l.unit.clone() },
                rate: l.rate,
                duration_minutes: l.duration_minutes,
                qty: Some(l.qty.unwrap_or(Decimal::ONE)),
                wastage_pct: l.wastage_pct,
                calculated_area: Some(calc.calculated_area),
                line_total: calc.line_total,
                mrp: l.mrp.filter(|m| *m > Decimal::ZERO),
                sort_order: Some(i32::try_from(i + 1).unwrap_or(i32::MAX)),
            }
        })
        .collect()
}

pub fn line_row(l: &ProductCostLine, sheet_id: &str) -> Value {
    let mut v = serde_json::to_value(l).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.remove("id");
        m.insert("sheet_id".into(), json!(sheet_id));
    }
    v
}

/// Update the row with `id`; without one, update the row whose `column` equals
/// `value`, or insert. Returns the row's id. Saving a product restores it from the Bin.
async fn save_row(
    table: &str,
    values: &Map<String, Value>,
    id: Option<&str>,
    column: &str,
    value: &str,
) -> Result<String, ApiError> {
    let mut values = values.clone();
    if table == "products" {
        values.insert("deleted_at".into(), Value::Null);
    }
    let id = match id.filter(|s| !s.is_empty()) {
        Some(id) => Some(id.to_string()),
        None => Q::from(table).select("id").eq(column, value).one::<Id>().await?.map(|r| r.id),
    };
    if let Some(id) = id {
        Q::from(table).eq("id", &id).update(&values, None).await?;
        return Ok(id);
    }
    let rows: Vec<Id> = db::insert_returning(table, &Value::Object(values)).await?;
    rows.into_iter().next().map(|r| r.id).ok_or_else(|| ApiError::Server(format!("Nothing came back from {table}.")))
}

/// Gives a product this cost sheet and lines, replacing what it had.
async fn copy_costing(
    product_id: &str,
    code: &str,
    sheet: &Map<String, Value>,
    lines: &[ProductCostLine],
) -> Result<(), ApiError> {
    let mut values = sheet.clone();
    values.insert("product_id".into(), json!(product_id));
    values.insert("product_code".into(), json!(code));
    let sheet_id = save_row("product_cost_sheets", &values, None, "product_id", product_id).await?;
    Q::from("product_cost_lines").eq("sheet_id", &sheet_id).delete().await?;
    if !lines.is_empty() {
        let rows: Vec<Value> = lines.iter().map(|l| line_row(l, &sheet_id)).collect();
        db::insert("product_cost_lines", &rows, None, None).await?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct Named {
    id: String,
    name: String,
    #[serde(default)]
    is_active: bool,
    #[serde(default, deserialize_with = "lenient::decimal")]
    default_rate: Decimal,
    #[serde(default)]
    unit: String,
}

fn same(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The row under `parent_id` named `name` (any case): found, re-activated or created.
async fn find_or_create(
    table: &str,
    parent_col: &str,
    parent_id: &str,
    name: &str,
    extra: Value,
) -> Result<Named, ApiError> {
    let rows: Vec<Named> = Q::from(table).select("*").eq(parent_col, parent_id).rows().await?;
    if let Some(hit) = rows.into_iter().find(|r| same(&r.name, name)) {
        if !hit.is_active {
            Q::from(table).eq("id", &hit.id).update(&json!({ "is_active": true }), None).await?;
        }
        return Ok(hit);
    }
    let mut row = json!({ parent_col: parent_id, "name": name.trim() });
    if let (Value::Object(r), Value::Object(e)) = (&mut row, extra) {
        r.extend(e);
    }
    db::insert_returning::<_, Named>(table, &row)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::Server(format!("Nothing came back from {table}.")))
}

/// Outsourced products type their bought-out items: each typed path goes into
/// the Rates master (created, or its rate and unit updated) and the line is
/// linked to it. Returns the linked lines and the varieties whose rate changed.
async fn link_bought_out(lines: Vec<ProductCostLine>) -> Result<(Vec<ProductCostLine>, Vec<String>), ApiError> {
    let bought: Vec<&ProductCostLine> = lines.iter().filter(|l| l.stage_code == "bought_out").collect();
    let blank = |s: &Option<String>| s.as_deref().is_none_or(|s| s.trim().is_empty());
    if let Some(l) = bought
        .iter()
        .find(|l| l.category_name.trim().is_empty() || blank(&l.subcategory_name) || blank(&l.variety_name))
    {
        let label = [l.item_name.as_str(), l.category_name.as_str()]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or("a bought-out line");
        return Err(ApiError::Invalid(format!(
            "Fill Category, Subcategory and Variety for \"{label}\" to save it to the Rates & Hierarchy Master."
        )));
    }
    if bought.is_empty() {
        return Ok((lines, Vec::new()));
    }
    let wrap = |e: ApiError| ApiError::Server(format!("Saving bought-out items to the master failed: {e}"));
    let stage =
        Q::from("cost_stages").select("id").eq("code", "bought_out").one::<Id>().await.map_err(wrap)?.ok_or_else(
            || ApiError::Server("The Bought Out Items stage is missing from the Rates & Hierarchy Master.".into()),
        )?;
    let mut repriced: Vec<String> = Vec::new();
    let mut linked = Vec::new();
    for l in lines {
        if l.stage_code != "bought_out" {
            linked.push(l);
            continue;
        }
        let (sub_name, var_name) =
            (l.subcategory_name.clone().unwrap_or_default(), l.variety_name.clone().unwrap_or_default());
        let cat = find_or_create("cost_categories", "stage_id", &stage.id, &l.category_name, json!({}))
            .await
            .map_err(wrap)?;
        let sub =
            find_or_create("cost_subcategories", "category_id", &cat.id, &sub_name, json!({})).await.map_err(wrap)?;
        let v = find_or_create(
            "cost_varieties",
            "subcategory_id",
            &sub.id,
            &var_name,
            json!({ "default_rate": l.rate, "unit": l.unit, "default_wastage_pct": l.wastage_pct }),
        )
        .await
        .map_err(wrap)?;
        if v.default_rate != l.rate || v.unit != l.unit {
            Q::from("cost_varieties")
                .eq("id", &v.id)
                .update(&json!({ "default_rate": l.rate, "unit": l.unit }), None)
                .await
                .map_err(wrap)?;
            if !repriced.contains(&v.id) {
                repriced.push(v.id.clone());
            }
        }
        linked.push(ProductCostLine {
            category_name: cat.name.clone(),
            subcategory_name: Some(sub.name.clone()),
            variety_name: Some(v.name.clone()),
            item_name: format!("{} {}", sub.name, v.name),
            cost_variety_id: Some(v.id),
            ..l
        });
    }
    Ok((linked, repriced))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct VarietyRow {
    id: String,
    name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    default_rate: Decimal,
    unit: String,
    #[serde(deserialize_with = "lenient::decimal")]
    default_wastage_pct: Decimal,
    cost_subcategories: Option<SubRef>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SubRef {
    name: String,
    cost_categories: Option<CatRef>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CatRef {
    name: String,
}

#[derive(Deserialize)]
struct SheetOf {
    sheet_id: String,
}

#[derive(Deserialize)]
struct ProductOf {
    product_id: String,
}

/// After master rates change: re-rate every saved line using those varieties,
/// recompute their sheets and reprice their products. Returns how many
/// products changed. Hampers and quotes keep their own saved prices.
pub async fn reprice_varieties(ids: &[String]) -> Result<usize, ApiError> {
    if ids.is_empty() {
        return Ok(0);
    }
    let varieties: Vec<VarietyRow> = Q::from("cost_varieties")
        .select("id,name,default_rate,unit,default_wastage_pct,cost_subcategories(name,cost_categories(name))")
        .in_list("id", ids)
        .rows()
        .await?;
    let used: Vec<SheetOf> =
        Q::from("product_cost_lines").select("sheet_id").in_list("cost_variety_id", ids).rows().await?;
    let sheet_ids: Vec<String> = used.into_iter().map(|u| u.sheet_id).collect::<HashSet<_>>().into_iter().collect();
    if sheet_ids.is_empty() {
        return Ok(0);
    }
    let lines: Vec<ProductCostLine> =
        Q::from("product_cost_lines").select("*").in_list("sheet_id", &sheet_ids).rows().await?;
    let sheets: Vec<SheetRef> = Q::from("product_cost_sheets").select("*").in_list("id", &sheet_ids).rows().await?;
    let products: Vec<PricedProduct> = Q::from("products").select("id,code,name,cost_price,default_sp").rows().await?;
    let costed: Vec<ProductOf> =
        Q::from("product_cost_sheets").select("product_id").not_null("product_id").rows().await?;
    let gst = load_settings().await?.gst_rate;

    let rates: Vec<MasterRate> = varieties
        .into_iter()
        .map(|v| MasterRate {
            id: v.id,
            name: v.name,
            default_rate: v.default_rate,
            unit: v.unit,
            default_wastage_pct: v.default_wastage_pct,
            subcategory_name: v.cost_subcategories.as_ref().map(|s| s.name.clone()),
            category_name: v.cost_subcategories.and_then(|s| s.cost_categories).map(|c| c.name),
        })
        .collect();
    let costed_alone: HashSet<String> = costed.into_iter().map(|c| c.product_id).collect();
    let plan = plan_reprice(&RepriceInput {
        rates: &rates,
        lines: &lines,
        sheets: &sheets,
        products: &products,
        costed_alone: &costed_alone,
        gst_rate: gst,
    });

    // Upsert on id updates the existing rows in one request per table.
    if !plan.lines.is_empty() {
        db::insert("product_cost_lines", &plan.lines, Some("id"), None).await?;
    }
    if !plan.products.is_empty() {
        db::insert("products", &plan.products, Some("id"), None).await?;
    }
    if !plan.sheets.is_empty() {
        db::insert("product_cost_sheets", &plan.sheets, Some("id"), None).await?;
    }
    Ok(plan.products.len())
}

/// A Decimal as a JSON number (jsonb columns hold numbers, as the old app wrote them).
fn number(d: Decimal) -> Value {
    serde_json::Number::from_f64(f(d)).map_or(Value::Null, Value::Number)
}

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

#[derive(Deserialize)]
struct LiveCode {
    id: String,
    code: String,
}

#[derive(Deserialize)]
struct SiblingRow {
    id: String,
    code: String,
    name: String,
}

pub async fn save_costing(p: Payload) -> Result<Saved, ApiError> {
    let code = p.code.trim().to_string();
    let name = p.name.trim().to_string();
    if code.is_empty() {
        return Err(ApiError::Invalid("Product code is required.".into()));
    }
    if name.is_empty() {
        return Err(ApiError::Invalid("Product name is required.".into()));
    }
    if p.colors.is_empty() {
        return Err(ApiError::Invalid("Pick a colour: every product is one colour.".into()));
    }

    // Outsourced products type their bought-out items; they go into the master here.
    let (lines, mut repriced) =
        if p.source.as_deref() == Some("Outsource") { link_bought_out(p.lines).await? } else { (p.lines, Vec::new()) };
    for id in save_line_wastage(&lines).await? {
        if !repriced.contains(&id) {
            repriced.push(id);
        }
    }
    let prepared = prepare_lines(&lines);
    // Only real, non-zero percentages are kept.
    let overheads: StageOverheads =
        p.stage_overheads.into_iter().map(|(k, v)| (k.to_lowercase(), v)).filter(|(_, v)| !v.is_zero()).collect();
    let totals = calculate_cost_sheet_totals(&prepared, p.markup_pct, &overheads);
    let raw_sp = if p.selling_price > Decimal::ZERO { p.selling_price } else { totals.calculated_sp };
    let gst = load_settings().await?.gst_rate;
    // Rounding must not take an SP within the MRP limit over it.
    let final_sp = round_sp(raw_sp, mrp_limit(&prepared, Some(&overheads), totals.total_cost, gst));
    let target_margin = dec(if f(final_sp) > 0.0 { (f(final_sp) - f(totals.total_cost)) / f(final_sp) } else { 0.0 });

    // 1. The product. Several colours means one product per colour: this one
    // takes the first, the rest become siblings in step 4.
    let product_values = obj(json!({
        "code": code,
        "name": name,
        "category_id": p.category_id.filter(|s| !s.is_empty()),
        "source": p.source.filter(|s| !s.is_empty()),
        "cost_price": totals.total_cost,
        "markup_pct": totals.markup_pct,
        "target_margin": target_margin,
        "default_sp": final_sp,
        "colors": p.colors.iter().take(1).collect::<Vec<_>>(),
        "is_active": p.is_active,
    }));
    let product_id = save_row("products", &product_values, p.product_id.as_deref(), "code", &code)
        .await
        .map_err(err("Error saving product"))?;

    // 2. Its cost sheet.
    let sheet_values = obj(json!({
        "product_name": name,
        "material_total": totals.material_total,
        "hardware_total": totals.hardware_total,
        "finishing_total": totals.finishing_total,
        "machine_total": totals.machine_total,
        "total_cost": totals.total_cost,
        "markup_pct": totals.markup_pct,
        "calculated_sp": final_sp,
        "stage_overheads": overheads.iter().map(|(k, v)| (k.clone(), number(*v))).collect::<Map<String, Value>>(),
        "notes": p.notes,
        "created_by": p.user_id,
    }));
    let mut own = sheet_values.clone();
    own.insert("product_id".into(), json!(product_id));
    own.insert("product_code".into(), json!(code));
    let sheet_id = save_row("product_cost_sheets", &own, p.sheet_id.as_deref(), "product_id", &product_id)
        .await
        .map_err(err("Error saving cost sheet"))?;

    // 3. Replace the lines.
    Q::from("product_cost_lines").eq("sheet_id", &sheet_id).delete().await?;
    if !prepared.is_empty() {
        let rows: Vec<Value> = prepared.iter().map(|l| line_row(l, &sheet_id)).collect();
        db::insert("product_cost_lines", &rows, None, None).await.map_err(err("Error saving cost lines"))?;
    }

    // 4. One sibling product per newly ticked colour, a copy of this one.
    let mut variants = vec![Variant {
        color: p.colors.first().cloned().unwrap_or_default(),
        code: code.clone(),
        id: product_id.clone(),
    }];
    let extra: Vec<_> =
        codes_for_colors(&code, p.colors.get(1..).unwrap_or_default()).into_iter().filter(|v| v.code != code).collect();
    if !extra.is_empty() {
        let codes: Vec<&str> = extra.iter().map(|v| v.code.as_str()).collect();
        let live: Vec<LiveCode> =
            Q::from("products").select("id,code").in_list("code", &codes).null("deleted_at").rows().await?;
        let live: HashMap<String, String> = live.into_iter().map(|r| (r.code, r.id)).collect();
        for v in extra {
            if let Some(id) = live.get(&v.code) {
                variants.push(Variant { color: v.color, code: v.code, id: id.clone() });
                continue;
            }
            let mut values = product_values.clone();
            values.insert("code".into(), json!(v.code));
            values.insert("colors".into(), json!([v.color]));
            let id = save_row("products", &values, None, "code", &v.code)
                .await
                .map_err(|e| ApiError::Server(format!("Error saving {}: {e}", v.code)))?;
            copy_costing(&id, &v.code, &sheet_values, &prepared).await?;
            variants.push(Variant { color: v.color, code: v.code, id });
        }
    }

    // 5. Other colours ticked in the save dialog take the same price and costing.
    let parsed = parse_product_code(&code);
    if !p.apply_to.is_empty() && !parsed.category_code.is_empty() && !parsed.serial.is_empty() {
        let price = json!({
            "cost_price": totals.total_cost,
            "markup_pct": totals.markup_pct,
            "target_margin": target_margin,
            "default_sp": final_sp,
        });
        let siblings: Vec<SiblingRow> = Q::from("products")
            .select("id,code,name")
            .in_list("id", &p.apply_to)
            .ilike("code", &format!("{}/{}/*", parsed.category_code, parsed.serial))
            .null("deleted_at")
            .rows()
            .await?;
        for s in siblings {
            Q::from("products")
                .eq("id", &s.id)
                .update(&price, None)
                .await
                .map_err(|e| ApiError::Server(format!("Error updating {}: {e}", s.code)))?;
            let mut sv = sheet_values.clone();
            sv.insert("product_name".into(), json!(s.name));
            copy_costing(&s.id, &s.code, &sv, &prepared)
                .await
                .map_err(|e| ApiError::Server(format!("Error updating {}: {e}", s.code)))?;
        }
    }

    // A rate or wastage changed in the master: reprice the other products using it.
    reprice_varieties(&repriced).await.map_err(err("Saved, but repricing other products failed"))?;

    Ok(Saved { product_id, variants, total_cost: totals.total_cost, selling_price: final_sp })
}

/// A wastage % edited on a line linked to a rate is saved to that rate on the
/// Rates page. Returns the rates changed, to reprice the products using them.
async fn save_line_wastage(lines: &[ProductCostLine]) -> Result<Vec<String>, ApiError> {
    #[derive(Deserialize)]
    struct Rate {
        id: String,
        default_wastage_pct: Decimal,
    }
    let mut typed: BTreeMap<String, Decimal> = BTreeMap::new();
    for l in lines {
        if let Some(id) = &l.cost_variety_id {
            typed.insert(id.clone(), l.wastage_pct);
        }
    }
    if typed.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<String> = typed.keys().cloned().collect();
    let rates: Vec<Rate> =
        Q::from("cost_varieties").select("id,default_wastage_pct").in_list("id", &ids).rows().await?;
    let changed: Vec<String> =
        rates.into_iter().filter(|r| typed.get(&r.id) != Some(&r.default_wastage_pct)).map(|r| r.id).collect();
    if changed.is_empty() {
        return Ok(changed);
    }
    if !crate::db::me().role.is_admin() {
        return Err(ApiError::Invalid(
            "Only an admin can change a rate's wastage, because it updates the products that use it.".into(),
        ));
    }
    for (id, pct) in typed.iter().filter(|(id, _)| changed.contains(id)) {
        Q::from("cost_varieties").eq("id", id).update(&json!({ "default_wastage_pct": pct }), None).await?;
    }
    Ok(changed)
}

/// The next free 4-digit serial for a category, across every product code.
pub async fn next_serial(category_code: &str) -> String {
    #[derive(Deserialize)]
    struct Code {
        code: String,
    }
    match Q::from("products").select("code").rows::<Code>().await {
        Ok(rows) => {
            let codes: Vec<String> = rows.into_iter().map(|r| r.code).collect();
            lattice_core::product_code::next_serial_for_category(category_code, &codes)
        }
        Err(_) => "0001".into(),
    }
}
