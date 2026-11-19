//! The Rates & Hierarchy Master's saves (ports of `master/*-actions.ts`).
//! Each returns the rows it changed, so the screen patches just those.

use crate::db::{self, Q, me};
use crate::pages::calculator::reprice_varieties;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::colors::{ProductColor, code_clash};
use lattice_core::cost_tree::{CostCategory, CostChange, CostSubcategory, CostVariety};
use lattice_core::lenient;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

pub type Saved = Result<(Vec<CostChange>, Option<String>), ApiError>;

#[derive(Deserialize)]
struct Id {
    id: String,
}

fn one<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, ApiError> {
    serde_json::from_value::<Vec<T>>(v)
        .ok()
        .and_then(|v| v.into_iter().next())
        .ok_or_else(|| ApiError::Server("The database returned nothing.".into()))
}

pub async fn add_category(stage_id: String, name: String) -> Saved {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Invalid("Category name is required.".into()));
    }
    let row: CostCategory = one(db::insert(
        "cost_categories",
        &json!({ "stage_id": stage_id, "name": name, "sort_order": 0 }),
        None,
        Some("*"),
    )
    .await?)?;
    Ok((vec![CostChange::Categories { upsert: vec![row], remove: vec![] }], None))
}

pub async fn rename_category(id: String, name: String) -> Saved {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(ApiError::Invalid("Category name cannot be empty.".into()));
    }
    #[derive(Deserialize)]
    struct Cat {
        name: String,
        stage_id: String,
    }
    let cat = Q::from("cost_categories")
        .select("name,stage_id")
        .eq("id", &id)
        .one::<Cat>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Category not found.".into()))?;
    if cat.name == trimmed {
        return Ok((vec![], None));
    }
    let taken = Q::from("cost_categories")
        .select("id")
        .eq("stage_id", &cat.stage_id)
        .ilike("name", &trimmed)
        .neq("id", &id)
        .one::<Id>()
        .await?;
    if taken.is_some() {
        return Err(ApiError::Conflict(format!("A category named \"{trimmed}\" already exists in this stage.")));
    }
    let renamed: CostCategory =
        one(Q::from("cost_categories").eq("id", &id).update(&json!({ "name": trimmed }), Some("*")).await?)?;
    // Lines keep a copy of the names: update the ones costed with this category's varieties.
    let subs: Vec<Id> = Q::from("cost_subcategories").select("id").eq("category_id", &id).rows().await?;
    let sub_ids: Vec<String> = subs.into_iter().map(|s| s.id).collect();
    if !sub_ids.is_empty() {
        let vars: Vec<Id> = Q::from("cost_varieties").select("id").in_list("subcategory_id", &sub_ids).rows().await?;
        let var_ids: Vec<String> = vars.into_iter().map(|v| v.id).collect();
        if !var_ids.is_empty() {
            Q::from("product_cost_lines")
                .in_list("cost_variety_id", &var_ids)
                .update(&json!({ "category_name": trimmed }), None)
                .await?;
        }
    }
    Ok((
        vec![CostChange::Categories { upsert: vec![renamed], remove: vec![] }],
        Some(format!("Category renamed to \"{trimmed}\".")),
    ))
}

pub async fn delete_category(id: String) -> Saved {
    Q::from("cost_categories").eq("id", &id).delete().await?;
    Ok((vec![CostChange::Categories { upsert: vec![], remove: vec![id] }], None))
}

pub async fn add_subcategory(category_id: String, name: String) -> Saved {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Invalid("Subcategory name is required (e.g. Birch, Acacia).".into()));
    }
    let row: CostSubcategory = one(db::insert(
        "cost_subcategories",
        &json!({ "category_id": category_id, "name": name, "sort_order": 0 }),
        None,
        Some("*"),
    )
    .await?)?;
    Ok((vec![CostChange::Subcategories { upsert: vec![row], remove: vec![] }], None))
}

pub async fn rename_subcategory(id: String, name: String) -> Saved {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(ApiError::Invalid("Subcategory name cannot be empty.".into()));
    }
    #[derive(Deserialize)]
    struct Sub {
        name: String,
        category_id: String,
    }
    let sub = Q::from("cost_subcategories")
        .select("name,category_id")
        .eq("id", &id)
        .one::<Sub>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Subcategory not found.".into()))?;
    if sub.name == trimmed {
        return Ok((vec![], None));
    }
    let taken = Q::from("cost_subcategories")
        .select("id")
        .eq("category_id", &sub.category_id)
        .ilike("name", &trimmed)
        .neq("id", &id)
        .one::<Id>()
        .await?;
    if taken.is_some() {
        return Err(ApiError::Conflict(format!("A subcategory named \"{trimmed}\" already exists in this category.")));
    }
    let renamed: CostSubcategory =
        one(Q::from("cost_subcategories").eq("id", &id).update(&json!({ "name": trimmed }), Some("*")).await?)?;
    #[derive(Deserialize)]
    struct Var {
        id: String,
        name: String,
    }
    let vars: Vec<Var> = Q::from("cost_varieties").select("id,name").eq("subcategory_id", &id).rows().await?;
    for v in vars {
        Q::from("product_cost_lines")
            .eq("cost_variety_id", &v.id)
            .update(&json!({ "subcategory_name": trimmed, "item_name": format!("{trimmed} {}", v.name) }), None)
            .await?;
    }
    Ok((
        vec![CostChange::Subcategories { upsert: vec![renamed], remove: vec![] }],
        Some(format!("Subcategory renamed to \"{trimmed}\".")),
    ))
}

pub async fn delete_subcategory(id: String) -> Saved {
    Q::from("cost_subcategories").eq("id", &id).delete().await?;
    Ok((vec![CostChange::Subcategories { upsert: vec![], remove: vec![id] }], None))
}

/// The fields of the variety dialog.
#[derive(Clone, Debug, Default)]
pub struct VarietyForm {
    pub id: Option<String>,
    pub subcategory_id: String,
    pub name: String,
    pub default_rate: Decimal,
    pub unit: String,
    pub default_wastage_pct: Decimal,
    pub notes: Option<String>,
    pub sort_order: i32,
}

/// Adds or edits a variety. A new name, rate, unit or wastage reprices every
/// product costed with it (admins only, since it changes their prices).
pub async fn save_variety(f: VarietyForm) -> Saved {
    if f.subcategory_id.is_empty() {
        return Err(ApiError::Invalid("Subcategory is required.".into()));
    }
    let name = f.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Invalid("Variety name is required (e.g. 8mm, 12mm).".into()));
    }
    let unit = Some(f.unit.trim().to_string()).filter(|u| !u.is_empty()).unwrap_or_else(|| "sq ft".into());
    let values = json!({
        "subcategory_id": f.subcategory_id,
        "name": name,
        "default_rate": f.default_rate,
        "unit": unit,
        "default_wastage_pct": f.default_wastage_pct,
        "notes": f.notes.as_ref().map(|n| n.trim()).filter(|n| !n.is_empty()),
        "sort_order": f.sort_order,
    });
    let Some(id) = f.id else {
        let row: CostVariety = one(db::insert("cost_varieties", &values, None, Some("*")).await?)?;
        return Ok((vec![CostChange::Varieties { upsert: vec![row], remove: vec![] }], None));
    };
    #[derive(Deserialize)]
    struct Before {
        #[serde(deserialize_with = "lenient::decimal")]
        default_rate: Decimal,
        unit: String,
        name: String,
        #[serde(deserialize_with = "lenient::decimal")]
        default_wastage_pct: Decimal,
    }
    let before = Q::from("cost_varieties")
        .select("default_rate,unit,name,default_wastage_pct")
        .eq("id", &id)
        .one::<Before>()
        .await?;
    let repricing = before.is_some_and(|b| {
        b.default_rate != f.default_rate
            || b.unit != unit
            || b.name != name
            || b.default_wastage_pct != f.default_wastage_pct
    });
    if repricing && !me().role.is_admin() {
        return Err(ApiError::Forbidden(
            "Only an admin can change a variety's name, rate, unit or wastage, because it updates the products that use it.".into(),
        ));
    }
    let saved: CostVariety = one(Q::from("cost_varieties").eq("id", &id).update(&values, Some("*")).await?)?;
    let changes = vec![CostChange::Varieties { upsert: vec![saved], remove: vec![] }];
    if !repricing {
        return Ok((changes, None));
    }
    db::cache_drop(&["products", "catalog"]).await;
    match reprice_varieties(&[id]).await {
        Ok(0) => Ok((changes, Some("Rate saved. No saved products use this variety yet.".into()))),
        Ok(n) => Ok((
            changes,
            Some(format!(
                "Rate saved. {n} product{} repriced; quotations and invoices keep their prices.",
                if n == 1 { "" } else { "s" }
            )),
        )),
        Err(e) => Err(ApiError::Server(format!("Rate saved, but repricing failed: {e}"))),
    }
}

pub async fn delete_variety(id: String) -> Saved {
    Q::from("cost_varieties").eq("id", &id).delete().await?;
    Ok((vec![CostChange::Varieties { upsert: vec![], remove: vec![id] }], None))
}

/// One past the highest sort order in a category, so a moved subcategory lands last.
async fn next_sort(category_id: &str) -> Result<i32, ApiError> {
    #[derive(Deserialize)]
    struct S {
        sort_order: i32,
    }
    let top = Q::from("cost_subcategories")
        .select("sort_order")
        .eq("category_id", category_id)
        .order_desc("sort_order")
        .one::<S>()
        .await?;
    Ok(top.map_or(0, |s| s.sort_order) + 1)
}

async fn name_taken(category_id: &str, name: &str) -> Result<bool, ApiError> {
    Ok(Q::from("cost_subcategories")
        .select("id")
        .eq("category_id", category_id)
        .ilike("name", name)
        .one::<Id>()
        .await?
        .is_some())
}

#[derive(Deserialize)]
struct SubRow {
    name: String,
    category_id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CatRow {
    name: String,
    cost_stages: Option<StageCode>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StageCode {
    code: String,
}

pub async fn move_subcategory(sub_id: String, target: String) -> Saved {
    if sub_id.is_empty() || target.is_empty() {
        return Err(ApiError::Invalid("Subcategory and destination category are required.".into()));
    }
    let sub = Q::from("cost_subcategories")
        .select("id,name,category_id")
        .eq("id", &sub_id)
        .one::<SubRow>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Subcategory not found.".into()))?;
    if sub.category_id == target {
        return Err(ApiError::Invalid("The subcategory is already in that category.".into()));
    }
    let cat = Q::from("cost_categories")
        .select("id,name,stage_id,cost_stages(code)")
        .eq("id", &target)
        .one::<CatRow>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Destination category not found.".into()))?;
    if name_taken(&target, &sub.name).await? {
        return Err(ApiError::Conflict(format!(
            "A subcategory named \"{}\" already exists in \"{}\". Please rename it first or choose another category.",
            sub.name, cat.name
        )));
    }
    let sort = next_sort(&target).await?;
    let moved: CostSubcategory = one(Q::from("cost_subcategories")
        .eq("id", &sub_id)
        .update(&json!({ "category_id": target, "sort_order": sort }), Some("*"))
        .await?)?;
    let vars: Vec<Id> = Q::from("cost_varieties").select("id").eq("subcategory_id", &sub_id).rows().await?;
    let var_ids: Vec<String> = vars.into_iter().map(|v| v.id).collect();
    if !var_ids.is_empty() {
        let mut patch = json!({ "category_name": cat.name });
        if let Some(code) = cat.cost_stages.as_ref().map(|s| s.code.clone()).filter(|c| !c.is_empty())
            && let Value::Object(map) = &mut patch
        {
            map.insert("stage_code".into(), json!(code));
        }
        Q::from("product_cost_lines").in_list("cost_variety_id", &var_ids).update(&patch, None).await?;
    }
    Ok((
        vec![CostChange::Subcategories { upsert: vec![moved], remove: vec![] }],
        Some(format!("Subcategory \"{}\" moved to \"{}\".", sub.name, cat.name)),
    ))
}

pub async fn duplicate_subcategory(sub_id: String, target: String, new_name: String) -> Saved {
    if sub_id.is_empty() || target.is_empty() {
        return Err(ApiError::Invalid("Subcategory and destination category are required.".into()));
    }
    let sub = Q::from("cost_subcategories")
        .select("id,name,category_id")
        .eq("id", &sub_id)
        .one::<SubRow>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Subcategory not found.".into()))?;
    let varieties: Vec<CostVariety> =
        Q::from("cost_varieties").select("*").eq("subcategory_id", &sub_id).rows().await?;
    let cat = Q::from("cost_categories")
        .select("id,name")
        .eq("id", &target)
        .one::<CatRow>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Destination category not found.".into()))?;
    let name = match new_name.trim() {
        "" if sub.category_id == target => format!("{} (Copy)", sub.name),
        "" => sub.name.clone(),
        n => n.to_string(),
    };
    if name_taken(&target, &name).await? {
        return Err(ApiError::Conflict(format!(
            "A subcategory named \"{name}\" already exists in \"{}\". Please pick a different name.",
            cat.name
        )));
    }
    let sort = next_sort(&target).await?;
    let new_sub: CostSubcategory = one(db::insert(
        "cost_subcategories",
        &json!({ "category_id": target, "name": name, "sort_order": sort, "is_active": true }),
        None,
        Some("*"),
    )
    .await?)?;
    let mut copies: Vec<CostVariety> = Vec::new();
    if !varieties.is_empty() {
        let rows: Vec<Value> = varieties
            .iter()
            .map(|v| {
                json!({
                    "subcategory_id": new_sub.id, "name": v.name, "default_rate": v.default_rate, "unit": v.unit,
                    "default_wastage_pct": v.default_wastage_pct, "notes": v.notes, "sort_order": v.sort_order, "is_active": v.is_active,
                })
            })
            .collect();
        match db::insert("cost_varieties", &rows, None, Some("*")).await {
            // Inactive source varieties are copied too, but the master shows only active ones.
            Ok(v) => {
                copies = serde_json::from_value::<Vec<CostVariety>>(v)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|v| v.is_active)
                    .collect()
            }
            Err(e) => {
                let _ = Q::from("cost_subcategories").eq("id", &new_sub.id).delete().await;
                return Err(e);
            }
        }
    }
    let message = format!("Subcategory \"{name}\" created in \"{}\" with {} varieties.", cat.name, varieties.len());
    Ok((
        vec![
            CostChange::Subcategories { upsert: vec![new_sub], remove: vec![] },
            CostChange::Varieties { upsert: copies, remove: vec![] },
        ],
        Some(message),
    ))
}

/// Brings every saved costing up to the master: lines whose rate, unit,
/// wastage or names differ from their variety are re-rated and repriced.
pub async fn sync_all_rates() -> Result<String, ApiError> {
    if !me().role.is_admin() {
        return Err(ApiError::Forbidden("Only an admin can sync products with the master rates.".into()));
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Stored {
        sheet_id: String,
        cost_variety_id: String,
        #[serde(deserialize_with = "lenient::decimal")]
        rate: Decimal,
        unit: String,
        #[serde(deserialize_with = "lenient::decimal")]
        wastage_pct: Decimal,
        stage_code: String,
        variety_name: Option<String>,
        subcategory_name: Option<String>,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Master {
        id: String,
        name: String,
        #[serde(deserialize_with = "lenient::decimal")]
        default_rate: Decimal,
        unit: String,
        #[serde(deserialize_with = "lenient::decimal")]
        default_wastage_pct: Decimal,
        cost_subcategories: Option<Named>,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Named {
        name: String,
    }
    let lines: Vec<Stored> = Q::from("product_cost_lines")
        .select("sheet_id,cost_variety_id,rate,unit,wastage_pct,stage_code,variety_name,subcategory_name")
        .not_null("cost_variety_id")
        .rows()
        .await?;
    let varieties: Vec<Master> = Q::from("cost_varieties")
        .select("id,name,default_rate,unit,default_wastage_pct,cost_subcategories(name)")
        .rows()
        .await?;
    let master: BTreeMap<String, Master> = varieties.into_iter().map(|v| (v.id.clone(), v)).collect();
    let stale: Vec<&Stored> = lines
        .iter()
        .filter(|l| {
            master.get(&l.cost_variety_id).is_some_and(|v| {
                let wastage = if l.stage_code == "bought_out" { Decimal::ZERO } else { v.default_wastage_pct };
                let sub = v
                    .cost_subcategories
                    .as_ref()
                    .map(|s| Some(s.name.clone()))
                    .unwrap_or_else(|| l.subcategory_name.clone());
                l.rate != v.default_rate
                    || l.unit != v.unit
                    || l.wastage_pct != wastage
                    || l.variety_name.as_deref() != Some(v.name.as_str())
                    || l.subcategory_name != sub
            })
        })
        .collect();
    if stale.is_empty() {
        return Ok("Every product already uses the current master rates and names.".into());
    }
    let ids: Vec<String> =
        stale.iter().map(|l| l.cost_variety_id.clone()).collect::<HashSet<_>>().into_iter().collect();
    reprice_varieties(&ids).await.map_err(|e| ApiError::Server(format!("Sync failed: {e}")))?;
    db::cache_drop(&["products", "catalog"]).await;
    let sheets = stale.iter().map(|l| &l.sheet_id).collect::<HashSet<_>>().len();
    Ok(format!(
        "Updated {} line{} on {sheets} product costing{}. Quotations and invoices keep their prices.",
        stale.len(),
        if stale.len() == 1 { "" } else { "s" },
        if sheets == 1 { "" } else { "s" }
    ))
}

/// Saves the colour list and each colour's swatch.
pub async fn save_colors(colors: &[ProductColor]) -> Result<(), ApiError> {
    let mut seen = HashSet::new();
    let cleaned: Vec<ProductColor> = colors
        .iter()
        .map(|c| ProductColor { name: c.name.trim().to_string(), ..c.clone() })
        .filter(|c| !c.name.is_empty() && seen.insert(c.name.to_lowercase()))
        .collect();
    // A colour's code ends product codes, so two colours can never share one.
    let names: Vec<&str> = cleaned.iter().map(|c| c.name.as_str()).collect();
    if let Some(clash) = code_clash(&names) {
        return Err(ApiError::Conflict(format!(
            "\"{}\" would get code {}, already used by {}. Pick a different name.",
            clash.name, clash.code, clash.with
        )));
    }
    let hex: serde_json::Map<String, Value> = cleaned.iter().map(|c| (c.name.clone(), json!(c.hex))).collect();
    let rows = json!([
        { "key": "product_colors", "value": names },
        { "key": "color_hex", "value": hex },
    ]);
    db::insert("app_settings", &rows, Some("key"), None).await?;
    db::cache_drop(&["settings"]).await;
    Ok(())
}
