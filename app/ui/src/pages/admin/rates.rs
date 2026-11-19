//! Rates from Excel (port of `cost-calculator/master/excel/*`): download the
//! hierarchy, change it, upload it back; new varieties are added and changed
//! rates reprice the products that use them.

use super::{Build, Check, Report, SheetWizard, stamp};
use crate::data::load_cost_rows;
use crate::db::{self, Q, insert, insert_returning, me};
use crate::pages::calculator::reprice_varieties;
use futures::FutureExt;
use lattice_core::api::ApiError;
use lattice_core::cost_tree::{CostCategory, CostRows, CostStage, CostSubcategory, CostVariety};
use lattice_core::import::Grid;
use lattice_core::sheets::rates::{plan_rate_import, rate_rows, rate_sheet};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use serde_json::{Value, json};
use std::rc::Rc;

fn admin_only() -> Result<(), ApiError> {
    if me().role.is_admin() { Ok(()) } else { Err(ApiError::Forbidden("Only an admin can change the master from Excel.".into())) }
}

/// Every row, inactive too: names are unique per parent either way.
/// ponytail: one request per table, so capped at the server's 1000 rows each;
/// past that a missed variety reads as new and its insert fails on the unique
/// name (safe, not silent). Page with offsets then.
async fn load_all_cost_rows() -> Result<CostRows, ApiError> {
    let all = |t: &str| Q::from(t).select("*").order("sort_order");
    let (st, ca, su, va) = (all("cost_stages"), all("cost_categories"), all("cost_subcategories"), all("cost_varieties"));
    let (stages, categories, subcategories, varieties) =
        futures::join!(st.rows::<CostStage>(), ca.rows::<CostCategory>(), su.rows::<CostSubcategory>(), va.rows::<CostVariety>());
    Ok(CostRows { stages: stages?, categories: categories?, subcategories: subcategories?, varieties: varieties? })
}

fn counts(c: &lattice_core::sheets::rates::RateCounts) -> Vec<(&'static str, usize)> {
    vec![
        ("newCategories", c.new_categories),
        ("newSubcategories", c.new_subcategories),
        ("newVarieties", c.new_varieties),
        ("updatedVarieties", c.updated_varieties),
    ]
}

/// Dry run unless `apply`.
async fn import_rate_sheet(grid: Grid, apply: bool, only_new: bool) -> Result<Report, ApiError> {
    admin_only()?;
    let mut plan = plan_rate_import(&grid, &load_all_cost_rows().await?, only_new);
    if !apply {
        return Ok(Report { ok: true, counts: counts(&plan.counts), warnings: plan.warnings, ..Default::default() });
    }
    for (table, list) in [("cost_categories", &plan.reactivate.categories), ("cost_subcategories", &plan.reactivate.subcategories)] {
        if !list.is_empty() {
            Q::from(table).in_list("id", list).update(&json!({ "is_active": true }), None).await?;
        }
    }
    // New categories, then subcategories under them, then varieties; each
    // insert's ids feed the next. Keys are "stageId|category|subcategory", lower case.
    #[derive(Deserialize)]
    struct NewCat {
        id: String,
        stage_id: String,
        name: String,
    }
    if !plan.new_categories.is_empty() {
        let rows: Vec<Value> =
            plan.new_categories.iter().map(|c| json!({ "stage_id": c.stage_id, "name": c.name, "sort_order": c.sort_order })).collect();
        for c in insert_returning::<_, NewCat>("cost_categories", &rows).await? {
            plan.ids.insert(format!("{}|{}", c.stage_id, c.name.trim().to_lowercase()), c.id);
        }
    }
    let key_by_id: std::collections::HashMap<String, String> = plan.ids.iter().map(|(k, v)| (v.clone(), k.clone())).collect();
    #[derive(Deserialize)]
    struct NewSub {
        id: String,
        category_id: String,
        name: String,
    }
    if !plan.new_subcategories.is_empty() {
        let rows: Vec<Value> = plan
            .new_subcategories
            .iter()
            .map(|s| json!({ "category_id": plan.ids.get(&s.cat_key), "name": s.name, "sort_order": s.sort_order }))
            .collect();
        for s in insert_returning::<_, NewSub>("cost_subcategories", &rows).await? {
            let parent = key_by_id.get(&s.category_id).cloned().unwrap_or_default();
            plan.ids.insert(format!("{parent}|{}", s.name.trim().to_lowercase()), s.id);
        }
    }
    if !plan.new_varieties.is_empty() {
        let rows: Vec<Value> = plan
            .new_varieties
            .iter()
            .map(|v| {
                json!({
                    "subcategory_id": plan.ids.get(&v.sub_key), "name": v.name, "default_rate": v.default_rate, "unit": v.unit,
                    "default_wastage_pct": v.default_wastage_pct, "notes": v.notes, "sort_order": v.sort_order,
                })
            })
            .collect();
        insert("cost_varieties", &rows, None, None).await?;
    }
    if !plan.updates.is_empty() {
        insert("cost_varieties", &plan.updates, Some("id"), None).await?;
    }
    let repriced = reprice_varieties(&plan.reprice).await;
    db::cache_drop(&["cost_rows", "products", "catalog"]).await;
    let counts = counts(&plan.counts);
    Ok(match repriced {
        Err(e) => Report { applied: true, counts, error: Some(format!("Master saved, but repricing failed: {e}")), ..Default::default() },
        Ok(n) => Report {
            error: None,
            ok: true,
            applied: true,
            counts,
            warnings: plan.warnings,
            added: plan.new_varieties.iter().map(|v| (v.label.clone(), None)).collect(),
            message: Some(if n > 0 {
                format!("Master saved. {n} product{} repriced; quotations and invoices keep their prices.", if n == 1 { "" } else { "s" })
            } else {
                "Master saved.".into()
            }),
        },
    })
}

/// `/cost-calculator/master/excel/add` and `/edit`.
#[component]
pub fn RateSheetPage() -> impl IntoView {
    let params = use_params_map();
    view! {
        {move || {
            let mode = params.with(|p| p.get("mode")).unwrap_or_default();
            if mode != "add" && mode != "edit" {
                return view! { <crate::pages::NotFound /> }.into_any();
            }
            let add = mode == "add";
            let build: Build = Rc::new(move || {
                async move {
                    admin_only()?;
                    let rows = if add { Vec::new() } else { rate_rows(&load_cost_rows().await?) };
                    Ok(vec![rate_sheet(rows).data()])
                }
                .boxed_local()
            });
            let check: Check = Rc::new(move |grid, apply, _| {
                async move { import_rate_sheet(grid, apply, add).await.unwrap_or_else(Report::failed) }.boxed_local()
            });
            let tips = if add {
                vec!["One row per variety: Type (e.g. Material), Category, Subcategory, Variety.", "New categories and subcategories are created for you.", "Varieties that already exist are skipped."]
            } else {
                vec!["Change rates, units, wastage or notes, or add rows at the bottom.", "Rename things in the master screen, not here. A blank cell keeps its value.", "A new rate, unit or wastage reprices the products that use it."]
            };
            let save_text: fn(&Report) -> String = if add {
                |r| { let n = r.count("newVarieties") + r.count("updatedVarieties"); format!("Add {n} variet{}", if n == 1 { "y" } else { "ies" }) }
            } else {
                |r| { let n = r.count("newVarieties") + r.count("updatedVarieties"); format!("Save {n} change{}", if n == 1 { "" } else { "s" }) }
            };
            view! {
                <crate::components::PageHeader title="Product Cost Calculator" subtitle="Work out what a product costs: materials, hardware, finishing and machine time">
                    <crate::pages::calculator::Tabs master=true />
                </crate::components::PageHeader>
                <div class="space-y-4">
                    <div class="flex items-center justify-between gap-3">
                        <h2 class="text-lg font-bold">{if add { "Bulk add rates" } else { "Bulk edit rates" }}</h2>
                        <A href="/cost-calculator/master" attr:class="text-sm font-semibold text-(--color-brand) underline">"Back to rates"</A>
                    </div>
                    <SheetWizard
                        tips=tips
                        download_label=if add { "Download template" } else { "Download rates" }
                        file_name=if add { "new-rates-template.xlsx".to_string() } else { format!("rates-and-hierarchy-{}.xlsx", stamp()) }
                        save_label=if add { "Add rates" } else { "Save changes" }
                        save_text=save_text
                        done_href="/cost-calculator/master"
                        done_label="Back to rates"
                        make=build
                        check=check
                    />
                </div>
            }
            .into_any()
        }}
    }
}
