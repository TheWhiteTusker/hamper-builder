//! The Rates & Hierarchy Master as one flat sheet, and reading it back
//! (port of `rate-sheet.ts` and `rate-import.ts`).

use super::{Cached, Cell, Column, Sheet, formula_row, list};
use crate::cost_tree::{CostRows, CostVariety, build_cost_tree};
use crate::import::{Grid, cell, numbered};
use crate::js;
use crate::numbers::parse_num;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const RATE_SHEET_NAME: &str = "Rates & Hierarchy";

fn columns() -> Vec<Column> {
    vec![
        Column::new("Type", 16),
        Column::new("Category", 22),
        Column::new("Subcategory", 22),
        Column::new("Variety / Specification", 26),
        Column::new("Rate", 12).money(),
        Column::new("Unit", 10),
        Column::new("Wastage %", 11),
        Column::new("Notes", 40),
        // Display only; the importer reads columns A–H.
        Column::new("Rate incl. wastage", 18).money().calc(|r| format!("IF(E{r}=\"\",\"\",E{r}*(1+N(G{r})/100))")),
    ]
}

/// A row per variety; a blank row (still carrying the formula) before each new subcategory.
pub fn rate_rows(db: &CostRows) -> Vec<Vec<Cell>> {
    let cols = columns();
    let incl = |r: usize| cols.get(8).and_then(|c| c.formula_at(r)).unwrap_or_default();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for stage in build_cost_tree(db) {
        for cat in &stage.categories {
            for sub in &cat.subcategories {
                if !rows.is_empty() && !sub.varieties.is_empty() {
                    rows.push(formula_row(&cols, rows.len() + 2));
                }
                for v in &sub.varieties {
                    let (rate, wastage) = (v.default_rate, v.default_wastage_pct);
                    let r = rows.len() + 2;
                    rows.push(vec![
                        Cell::text(&stage.row.name),
                        Cell::text(&cat.row.name),
                        Cell::text(&sub.row.name),
                        Cell::text(&v.name),
                        rate.into(),
                        Cell::text(&v.unit),
                        wastage.into(),
                        Cell::text(v.notes.clone().unwrap_or_default()),
                        Cell::formula(incl(r), Cached::Num(js::dec(js::f(rate) * (1.0 + js::f(wastage) / 100.0)))),
                    ]);
                }
            }
        }
    }
    rows
}

pub fn rate_sheet(rows: Vec<Vec<Cell>>) -> Sheet {
    Sheet { name: RATE_SHEET_NAME.into(), columns: columns(), rows, blank_rows: 300 }
}

/// Same defaults as the Add Variety dialog.
const NEW_UNIT: &str = "sq ft";
const NEW_WASTAGE: Decimal = dec!(10);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewCategory {
    pub key: String,
    pub stage_id: String,
    pub name: String,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewSubcategory {
    pub key: String,
    pub cat_key: String,
    pub name: String,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewVariety {
    pub sub_key: String,
    pub label: String,
    pub name: String,
    pub default_rate: Decimal,
    pub unit: String,
    pub default_wastage_pct: Decimal,
    pub notes: Option<String>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reactivate {
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateCounts {
    pub new_categories: usize,
    pub new_subcategories: usize,
    pub new_varieties: usize,
    pub updated_varieties: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RatePlan {
    /// Existing category / subcategory keys → ids; inserts add to it.
    pub ids: HashMap<String, String>,
    pub new_categories: Vec<NewCategory>,
    pub new_subcategories: Vec<NewSubcategory>,
    pub new_varieties: Vec<NewVariety>,
    pub updates: Vec<CostVariety>,
    pub reactivate: Reactivate,
    /// Varieties whose rate, unit or wastage changed: their products are repriced.
    pub reprice: Vec<String>,
    pub warnings: Vec<String>,
    pub counts: RateCounts,
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

#[derive(Default)]
struct SortCounter(HashMap<String, i32>);

impl SortCounter {
    fn bump(&mut self, parent: &str, sort: i32) {
        let n = self.0.entry(parent.to_string()).or_insert(0);
        *n = (*n).max(sort + 1);
    }

    fn take(&mut self, parent: &str) -> i32 {
        let n = self.0.entry(parent.to_string()).or_insert(0);
        *n += 1;
        *n - 1
    }
}

/// Reads the Rates & Hierarchy sheet back. Rows match on their name path in
/// any case (renaming is done in the master screen). `db` must hold every
/// row, inactive ones too: names are unique per parent either way. With
/// `only_new`, existing varieties are skipped rather than updated.
pub fn plan_rate_import(grid: &Grid, db: &CostRows, only_new: bool) -> RatePlan {
    let mut stages = HashMap::new();
    for s in &db.stages {
        stages.insert(norm(&s.name), s);
        stages.insert(norm(&s.code), s);
    }
    let mut plan = RatePlan::default();
    let mut key_of: HashMap<&str, String> = HashMap::new();
    let mut inactive: HashSet<String> = HashSet::new();
    let mut sorts = SortCounter::default();

    for c in &db.categories {
        let k = format!("{}|{}", c.stage_id, norm(&c.name));
        plan.ids.insert(k.clone(), c.id.clone());
        key_of.insert(&c.id, k);
        if !c.is_active {
            inactive.insert(c.id.clone());
        }
        sorts.bump(&c.stage_id, c.sort_order);
    }
    for s in &db.subcategories {
        let Some(parent) = key_of.get(s.category_id.as_str()).cloned() else { continue };
        let k = format!("{parent}|{}", norm(&s.name));
        plan.ids.insert(k.clone(), s.id.clone());
        key_of.insert(&s.id, k);
        if !s.is_active {
            inactive.insert(s.id.clone());
        }
        sorts.bump(&parent, s.sort_order);
    }
    let mut varieties: HashMap<String, &CostVariety> = HashMap::new();
    for v in &db.varieties {
        let Some(parent) = key_of.get(v.subcategory_id.as_str()).cloned() else { continue };
        varieties.insert(format!("{parent}|{}", norm(&v.name)), v);
        sorts.bump(&parent, v.sort_order);
    }

    let mut incomplete: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut repeated: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (row, n) in numbered(Some(grid)) {
        let c: Vec<String> = (0..8).map(|i| cell(Some(row), i)).collect();
        let [ty, cat, sub, name, rate, unit, wastage, notes] = c.as_slice() else { continue };
        if ty.is_empty() || cat.is_empty() || sub.is_empty() || name.is_empty() {
            incomplete.push(n.to_string());
            continue;
        }
        let Some(stage) = stages.get(&norm(ty)) else {
            if !unknown.contains(ty) {
                unknown.push(ty.clone());
            }
            continue;
        };

        let cat_key = format!("{}|{}", stage.id, norm(cat));
        if plan.ids.contains_key(&cat_key) {
            if let Some(id) = plan.ids.get(&cat_key)
                && inactive.remove(id)
            {
                plan.reactivate.categories.push(id.clone());
            }
        } else if !plan.new_categories.iter().any(|c| c.key == cat_key) {
            let sort_order = sorts.take(&stage.id);
            plan.new_categories.push(NewCategory {
                key: cat_key.clone(),
                stage_id: stage.id.clone(),
                name: cat.clone(),
                sort_order,
            });
        }

        let sub_key = format!("{cat_key}|{}", norm(sub));
        if plan.ids.contains_key(&sub_key) {
            if let Some(id) = plan.ids.get(&sub_key)
                && inactive.remove(id)
            {
                plan.reactivate.subcategories.push(id.clone());
            }
        } else if !plan.new_subcategories.iter().any(|s| s.key == sub_key) {
            let sort_order = sorts.take(&cat_key);
            plan.new_subcategories.push(NewSubcategory {
                key: sub_key.clone(),
                cat_key: cat_key.clone(),
                name: sub.clone(),
                sort_order,
            });
        }

        let key = format!("{sub_key}|{}", norm(name));
        let label = format!("{cat} › {sub} › {name}");
        if !seen.insert(key.clone()) {
            repeated.push(label);
            continue;
        }

        let Some(existing) = varieties.get(&key) else {
            let sort_order = sorts.take(&sub_key);
            plan.new_varieties.push(NewVariety {
                sub_key,
                label: format!("{} › {label}", stage.name),
                name: name.clone(),
                default_rate: parse_num(rate),
                unit: if unit.is_empty() { NEW_UNIT.into() } else { unit.clone() },
                default_wastage_pct: if wastage.is_empty() { NEW_WASTAGE } else { parse_num(wastage) },
                notes: (!notes.is_empty()).then(|| notes.clone()),
                sort_order,
            });
            continue;
        };
        if only_new {
            skipped.push(label);
            continue;
        }

        // A blank cell leaves that value as it is.
        let next = CostVariety {
            default_rate: if rate.is_empty() { existing.default_rate } else { parse_num(rate) },
            unit: if unit.is_empty() { existing.unit.clone() } else { unit.clone() },
            default_wastage_pct: if wastage.is_empty() { existing.default_wastage_pct } else { parse_num(wastage) },
            notes: if notes.is_empty() { existing.notes.clone() } else { Some(notes.clone()) },
            is_active: true,
            ..(*existing).clone()
        };
        let repricing = next.default_rate != existing.default_rate
            || next.unit != existing.unit
            || next.default_wastage_pct != existing.default_wastage_pct;
        if repricing || next.notes != existing.notes || !existing.is_active {
            plan.updates.push(next);
        }
        if repricing {
            plan.reprice.push(existing.id.clone());
        }
    }

    let w = &mut plan.warnings;
    if !incomplete.is_empty() {
        w.push(format!(
            "Rows missing a Type, Category, Subcategory or Variety were skipped: row {}",
            list(&incomplete)
        ));
    }
    if !unknown.is_empty() {
        let names: Vec<&str> = db.stages.iter().filter(|s| s.is_active).map(|s| s.name.as_str()).collect();
        w.push(format!("Unknown Type, rows skipped: {}. Use one of: {}", list(&unknown), names.join(", ")));
    }
    if !repeated.is_empty() {
        w.push(format!("Listed more than once, only the first is used: {}", list(&repeated)));
    }
    if !skipped.is_empty() {
        w.push(format!(
            "{} already exist and were skipped. Use Bulk edit to change them: {}",
            skipped.len(),
            list(&skipped)
        ));
    }
    if !plan.reprice.is_empty() {
        w.push(format!(
            "{} rate, unit or wastage change(s): products costed with them are repriced on save.",
            plan.reprice.len()
        ));
    }
    plan.counts = RateCounts {
        new_categories: plan.new_categories.len(),
        new_subcategories: plan.new_subcategories.len(),
        new_varieties: plan.new_varieties.len(),
        updated_varieties: plan.updates.len(),
    };
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost_tree::{CostCategory, CostStage, CostSubcategory};
    use crate::sheets::testing::grid;

    fn db() -> CostRows {
        CostRows {
            stages: vec![CostStage {
                id: "s1".into(),
                code: "material".into(),
                name: "Material".into(),
                sort_order: 1,
                is_active: true,
                ..Default::default()
            }],
            categories: vec![CostCategory {
                id: "c1".into(),
                stage_id: "s1".into(),
                name: "Woodbased".into(),
                sort_order: 0,
                is_active: true,
            }],
            subcategories: vec![CostSubcategory {
                id: "u1".into(),
                category_id: "c1".into(),
                name: "Birch".into(),
                sort_order: 0,
                is_active: true,
            }],
            varieties: vec![
                variety("v1", "u1", "8mm", dec!(100), Some("keep"), 0),
                variety("v2", "u1", "12mm", dec!(150), None, 1),
            ],
        }
    }

    fn variety(id: &str, sub: &str, name: &str, rate: Decimal, notes: Option<&str>, sort_order: i32) -> CostVariety {
        CostVariety {
            id: id.into(),
            subcategory_id: sub.into(),
            name: name.into(),
            default_rate: rate,
            unit: "sq ft".into(),
            default_wastage_pct: dec!(10),
            notes: notes.map(Into::into),
            sort_order,
            is_active: true,
        }
    }

    /// The export as it reads back, plus typed extra rows.
    fn round_trip(db: &CostRows, extra: &[&[&str]]) -> Grid {
        let mut g = grid(&rate_sheet(rate_rows(db)));
        g.extend(extra.iter().map(|r| r.iter().map(|c| (*c).to_string()).collect()));
        g
    }

    #[test]
    fn an_untouched_export_changes_nothing() {
        let plan = plan_rate_import(&round_trip(&db(), &[]), &db(), false);
        assert_eq!(plan.counts, RateCounts::default());
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    #[test]
    fn edits_update_by_name_path_and_new_rows_build_the_hierarchy() {
        let mut g = round_trip(&db(), &[&["material", "Metal", "Steel", "2mm", "80", "", "", ""]]);
        g[1][4] = "1,250.00".into();
        g[1][7] = String::new();
        g[2][3] = "12MM".into();
        let plan = plan_rate_import(&g, &db(), false);
        assert_eq!(plan.updates.len(), 1);
        assert_eq!(plan.updates[0].default_rate, dec!(1250));
        assert_eq!(plan.updates[0].notes.as_deref(), Some("keep"));
        assert_eq!(plan.reprice, ["v1"]);
        let cats: Vec<_> =
            plan.new_categories.iter().map(|c| (c.stage_id.as_str(), c.name.as_str(), c.sort_order)).collect();
        assert_eq!(cats, [("s1", "Metal", 1)]);
        assert_eq!(plan.new_subcategories.len(), 1);
        assert_eq!(plan.new_varieties[0].unit, "sq ft");
        assert_eq!(plan.new_varieties[0].default_wastage_pct, dec!(10));
    }

    #[test]
    fn add_mode_skips_existing_and_reports_unknown_types() {
        let mut g = round_trip(&db(), &[&["Plastic", "X", "Y", "Z", "1", "", "", ""]]);
        g[1][4] = "999".into();
        let plan = plan_rate_import(&g, &db(), true);
        assert!(plan.updates.is_empty() && plan.new_varieties.is_empty());
        assert_eq!(plan.warnings.len(), 2, "{:?}", plan.warnings);
    }

    #[test]
    fn a_blank_row_comes_before_each_new_subcategory() {
        let mut two = db();
        two.subcategories.push(CostSubcategory {
            id: "u2".into(),
            category_id: "c1".into(),
            name: "Pine".into(),
            sort_order: 1,
            is_active: true,
        });
        two.varieties
            .push(CostVariety { default_wastage_pct: dec!(5), ..variety("v3", "u2", "6mm", dec!(60), None, 0) });
        let rows = rate_rows(&two);
        let names: Vec<_> = rows.iter().map(|r| r.get(3).cloned().unwrap_or(Cell::Empty)).collect();
        assert_eq!(names, [Cell::text("8mm"), Cell::text("12mm"), Cell::Empty, Cell::text("6mm")]);
        let gap_formula = match rows.get(2).and_then(|r| r.get(8)) {
            Some(Cell::Formula { f, v: None }) => f.clone(),
            other => panic!("gap row lost its formula: {other:?}"),
        };
        assert_eq!(gap_formula, r#"IF(E4="","",E4*(1+N(G4)/100))"#);
        let g = round_trip(&two, &[&["Material", "", "", "", "", "", "", ""]]);
        let plan = plan_rate_import(&g, &two, false);
        assert_eq!(plan.warnings, ["Rows missing a Type, Category, Subcategory or Variety were skipped: row 6"]);
        assert_eq!(plan.updates.len() + plan.new_varieties.len(), 0);
    }
}
