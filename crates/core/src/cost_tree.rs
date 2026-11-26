//! The rates hierarchy: stage → category → subcategory → variety (port of `cost-tree.ts`).

use crate::lenient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculationType {
    #[default]
    Dimension,
    Piece,
    TimeBased,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostStage {
    pub id: String,
    pub code: String,
    pub name: String,
    pub calculation_type: CalculationType,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostCategory {
    pub id: String,
    pub stage_id: String,
    pub name: String,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostSubcategory {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CostVariety {
    pub id: String,
    pub subcategory_id: String,
    /// e.g. "8mm", "12mm", "10x2mm"
    pub name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_rate: Decimal,
    pub unit: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_wastage_pct: Decimal,
    pub notes: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// The hierarchy as the four flat tables it is stored in.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostRows {
    pub stages: Vec<CostStage>,
    pub categories: Vec<CostCategory>,
    pub subcategories: Vec<CostSubcategory>,
    pub varieties: Vec<CostVariety>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubcategoryNode {
    #[serde(flatten)]
    pub row: CostSubcategory,
    pub varieties: Vec<CostVariety>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CategoryNode {
    #[serde(flatten)]
    pub row: CostCategory,
    pub subcategories: Vec<SubcategoryNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageNode {
    #[serde(flatten)]
    pub row: CostStage,
    pub categories: Vec<CategoryNode>,
}

/// Rows in `sort_order`, stable so rows added locally sit after existing ones.
fn sorted<T: Clone>(rows: &[T], key: impl Fn(&T) -> i32) -> Vec<T> {
    let mut v = rows.to_vec();
    v.sort_by_key(|r| key(r));
    v
}

fn group<T>(rows: Vec<T>, parent: impl Fn(&T) -> String) -> HashMap<String, Vec<T>> {
    let mut map: HashMap<String, Vec<T>> = HashMap::new();
    for r in rows {
        map.entry(parent(&r)).or_default().push(r);
    }
    map
}

/// Nests the four tables by parent id, each level in `sort_order`.
pub fn build_cost_tree(rows: &CostRows) -> Vec<StageNode> {
    let mut varieties = group(sorted(&rows.varieties, |v| v.sort_order), |v| v.subcategory_id.clone());
    let subs = sorted(&rows.subcategories, |s| s.sort_order)
        .into_iter()
        .map(|row| SubcategoryNode { varieties: varieties.remove(&row.id).unwrap_or_default(), row })
        .collect();
    let mut subs = group(subs, |s: &SubcategoryNode| s.row.category_id.clone());
    let cats = sorted(&rows.categories, |c| c.sort_order)
        .into_iter()
        .map(|row| CategoryNode { subcategories: subs.remove(&row.id).unwrap_or_default(), row })
        .collect();
    let mut cats = group(cats, |c: &CategoryNode| c.row.stage_id.clone());
    sorted(&rows.stages, |s| s.sort_order)
        .into_iter()
        .map(|row| StageNode { categories: cats.remove(&row.id).unwrap_or_default(), row })
        .collect()
}

/// Rows a Master save wrote (replaced by id, or added) and ids it deleted, for one table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "table", rename_all = "snake_case")]
pub enum CostChange {
    Categories {
        #[serde(default)]
        upsert: Vec<CostCategory>,
        #[serde(default)]
        remove: Vec<String>,
    },
    Subcategories {
        #[serde(default)]
        upsert: Vec<CostSubcategory>,
        #[serde(default)]
        remove: Vec<String>,
    },
    Varieties {
        #[serde(default)]
        upsert: Vec<CostVariety>,
        #[serde(default)]
        remove: Vec<String>,
    },
}

trait HasId {
    fn id(&self) -> &str;
}
impl HasId for CostCategory {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for CostSubcategory {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for CostVariety {
    fn id(&self) -> &str {
        &self.id
    }
}

fn patch<T: HasId + Clone>(list: &[T], upsert: &[T], remove: &[String]) -> Vec<T> {
    let mut next: Vec<T> = list
        .iter()
        .filter(|r| !remove.iter().any(|id| id == r.id()))
        .map(|r| upsert.iter().find(|u| u.id() == r.id()).unwrap_or(r).clone())
        .collect();
    next.extend(upsert.iter().filter(|u| !list.iter().any(|r| r.id() == u.id())).cloned());
    next
}

/// The rows after a save, touching only what the save changed.
pub fn apply_cost_changes(rows: &CostRows, changes: &[CostChange]) -> CostRows {
    let mut next = rows.clone();
    for c in changes {
        match c {
            CostChange::Categories { upsert, remove } => next.categories = patch(&next.categories, upsert, remove),
            CostChange::Subcategories { upsert, remove } => {
                next.subcategories = patch(&next.subcategories, upsert, remove);
            }
            CostChange::Varieties { upsert, remove } => next.varieties = patch(&next.varieties, upsert, remove),
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn cat(id: &str, sort_order: i32) -> CostCategory {
        CostCategory { id: id.into(), stage_id: "s1".into(), name: id.into(), sort_order, is_active: true }
    }
    fn sub(id: &str, category_id: &str, sort_order: i32) -> CostSubcategory {
        CostSubcategory { id: id.into(), category_id: category_id.into(), name: id.into(), sort_order, is_active: true }
    }
    fn variety(id: &str, subcategory_id: &str, rate: Decimal) -> CostVariety {
        CostVariety {
            id: id.into(),
            subcategory_id: subcategory_id.into(),
            name: id.into(),
            default_rate: rate,
            unit: "sq ft".into(),
            is_active: true,
            ..Default::default()
        }
    }

    fn rows() -> CostRows {
        CostRows {
            stages: vec![CostStage {
                id: "s1".into(),
                code: "material".into(),
                name: "Material".into(),
                is_active: true,
                ..Default::default()
            }],
            categories: vec![cat("wood", 1), cat("metal", 2)],
            subcategories: vec![sub("birch", "wood", 0), sub("steel", "metal", 0)],
            varieties: vec![variety("8mm", "birch", dec!(10)), variety("2mm", "steel", dec!(10))],
        }
    }

    fn names(r: &CostRows) -> Vec<String> {
        let tree = build_cost_tree(r);
        tree[0]
            .categories
            .iter()
            .map(|c| {
                let subs: Vec<String> = c
                    .subcategories
                    .iter()
                    .map(|s| {
                        format!(
                            "{}[{}]",
                            s.row.id,
                            s.varieties.iter().map(|v| v.id.as_str()).collect::<Vec<_>>().join(",")
                        )
                    })
                    .collect();
                format!("{}: {}", c.row.id, subs.join(","))
            })
            .collect()
    }

    #[test]
    fn nests_by_parent_in_sort_order() {
        assert_eq!(names(&rows()), ["wood: birch[8mm]", "metal: steel[2mm]"]);
        let flipped = CostRows { categories: vec![cat("wood", 3), cat("metal", 2)], ..rows() };
        assert_eq!(names(&flipped), ["metal: steel[2mm]", "wood: birch[8mm]"]);
    }

    #[test]
    fn upsert_replaces_in_place() {
        let r = rows();
        let next = apply_cost_changes(
            &r,
            &[CostChange::Varieties { upsert: vec![variety("8mm", "birch", dec!(99))], remove: vec![] }],
        );
        assert_eq!(next.varieties[0].default_rate, dec!(99));
        assert_eq!(next.varieties[1], r.varieties[1]);
        assert_eq!(next.categories, r.categories);
        assert_eq!(r.varieties[0].default_rate, dec!(10), "input not mutated");
    }

    #[test]
    fn upsert_of_a_new_id_appends() {
        let next =
            apply_cost_changes(&rows(), &[CostChange::Categories { upsert: vec![cat("glass", 2)], remove: vec![] }]);
        assert_eq!(names(&next), ["wood: birch[8mm]", "metal: steel[2mm]", "glass: "]);
    }

    #[test]
    fn remove_drops_the_row_and_its_subtree() {
        let next =
            apply_cost_changes(&rows(), &[CostChange::Categories { upsert: vec![], remove: vec!["wood".into()] }]);
        assert_eq!(names(&next), ["metal: steel[2mm]"]);
    }

    #[test]
    fn moving_a_subcategory_is_an_upsert() {
        let next = apply_cost_changes(
            &rows(),
            &[CostChange::Subcategories { upsert: vec![sub("birch", "metal", 1)], remove: vec![] }],
        );
        assert_eq!(names(&next), ["wood: ", "metal: steel[2mm],birch[8mm]"]);
    }

    #[test]
    fn duplicating_adds_subcategory_and_varieties() {
        let next = apply_cost_changes(
            &rows(),
            &[
                CostChange::Subcategories { upsert: vec![sub("birch-copy", "wood", 1)], remove: vec![] },
                CostChange::Varieties { upsert: vec![variety("8mm-copy", "birch-copy", dec!(10))], remove: vec![] },
            ],
        );
        assert_eq!(names(&next), ["wood: birch[8mm],birch-copy[8mm-copy]", "metal: steel[2mm]"]);
    }

    #[test]
    fn change_json_matches_the_ts_shape() -> Result<(), serde_json::Error> {
        let c: CostChange = serde_json::from_str(r#"{"table":"varieties","remove":["v1"]}"#)?;
        assert_eq!(c, CostChange::Varieties { upsert: vec![], remove: vec!["v1".into()] });
        Ok(())
    }
}
