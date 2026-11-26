//! Repricing after the rates master changes (port of `reprice-plan.ts`).
//!
//! Pure: works out the rows to write. Phase 2 sends the plan to the
//! `apply_cost_plan` RPC so it commits in one transaction.

use crate::costing::{ProductCostLine, StageOverheads, calculate_cost_sheet_totals, calculate_line_cost, scaled_price};
use crate::js::f;
use crate::lenient;
use crate::mrp::mrp_limit;
use crate::product_code::parse_product_code;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MasterRate {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_rate: Decimal,
    pub unit: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_wastage_pct: Decimal,
    pub subcategory_name: Option<String>,
    pub category_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PricedProduct {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost_price: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_sp: Decimal,
}

/// The `product_cost_sheets` columns the reprice reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SheetRef {
    pub id: String,
    pub product_id: Option<String>,
    pub product_code: Option<String>,
    #[serde(deserialize_with = "lenient::null_default")]
    pub product_name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub markup_pct: Decimal,
    #[serde(deserialize_with = "lenient::decimal_map")]
    pub stage_overheads: StageOverheads,
}

pub struct RepriceInput<'a> {
    pub rates: &'a [MasterRate],
    /// Every line of the affected sheets.
    pub lines: &'a [ProductCostLine],
    pub sheets: &'a [SheetRef],
    pub products: &'a [PricedProduct],
    /// Products costed on a sheet of their own, so not priced as a sibling.
    pub costed_alone: &'a HashSet<String>,
    /// GST for the MRP limit: a repriced SP never goes past it.
    pub gst_rate: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub id: String,
    pub code: String,
    pub name: String,
    pub cost_price: Decimal,
    pub default_sp: Decimal,
    pub target_margin: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetUpdate {
    pub id: String,
    pub product_name: String,
    pub material_total: Decimal,
    pub hardware_total: Decimal,
    pub finishing_total: Decimal,
    pub machine_total: Decimal,
    pub total_cost: Decimal,
    pub calculated_sp: Decimal,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RepricePlan {
    pub lines: Vec<ProductCostLine>,
    pub sheets: Vec<SheetUpdate>,
    pub products: Vec<ProductUpdate>,
}

/// The products a sheet prices: its own, plus colour siblings saved from it
/// (LC/0001/WL → LC/0001/BL) unless one has since been costed on its own sheet.
fn products_of_sheet(sheet: &SheetRef, products: &[PricedProduct], costed_alone: &HashSet<String>) -> HashSet<String> {
    let mut ids: HashSet<String> = sheet.product_id.iter().cloned().collect();
    let parsed = parse_product_code(sheet.product_code.as_deref().unwrap_or_default());
    if parsed.is_valid {
        let prefix = format!("{}/{}/", parsed.category_code, parsed.serial);
        ids.extend(
            products
                .iter()
                .filter(|p| p.code.starts_with(&prefix) && !costed_alone.contains(&p.id))
                .map(|p| p.id.clone()),
        );
    }
    ids
}

/// Re-rates every line using one of `rates`, recomputes their sheets (with
/// overheads) and reprices those sheets' products, scaling each SP with its
/// cost so the margin holds.
pub fn plan_reprice(input: &RepriceInput) -> RepricePlan {
    let rates: HashMap<&str, &MasterRate> = input.rates.iter().map(|r| (r.id.as_str(), r)).collect();

    let lines: Vec<(ProductCostLine, bool)> = input
        .lines
        .iter()
        .map(|l| {
            let Some(v) = l.cost_variety_id.as_deref().and_then(|id| rates.get(id)) else {
                return (l.clone(), false);
            };
            let wastage_pct = v.default_wastage_pct;
            let mut line = ProductCostLine { rate: v.default_rate, unit: v.unit.clone(), wastage_pct, ..l.clone() };
            let calc = calculate_line_cost(&line);
            // Lines also take the master's current names, so renames show everywhere.
            if let Some(c) = v.category_name.as_ref().filter(|c| !c.is_empty()) {
                line.category_name.clone_from(c);
            }
            if let Some(s) = v.subcategory_name.as_ref().filter(|s| !s.is_empty()) {
                line.subcategory_name = Some(s.clone());
                line.item_name = format!("{s} {}", v.name);
            }
            line.variety_name = Some(v.name.clone());
            line.calculated_area = Some(calc.calculated_area);
            line.line_total = calc.line_total;
            (line, true)
        })
        .collect();

    let changed_sheets: HashSet<Option<&str>> =
        lines.iter().filter(|(_, changed)| *changed).map(|(l, _)| l.sheet_id.as_deref()).collect();

    let mut products: Vec<ProductUpdate> = Vec::new();
    let mut sheets = Vec::new();

    for sheet in input.sheets.iter().filter(|s| changed_sheets.contains(&Some(s.id.as_str()))) {
        let sheet_lines: Vec<ProductCostLine> = lines
            .iter()
            .filter(|(l, _)| l.sheet_id.as_deref() == Some(sheet.id.as_str()))
            .map(|(l, _)| l.clone())
            .collect();
        let t = calculate_cost_sheet_totals(&sheet_lines, sheet.markup_pct, &sheet.stage_overheads);
        let cap = mrp_limit(&sheet_lines, Some(&sheet.stage_overheads), t.total_cost, input.gst_rate);
        let ids = products_of_sheet(sheet, input.products, input.costed_alone);

        for p in input.products.iter().filter(|p| ids.contains(&p.id)) {
            let scaled = scaled_price(p.cost_price, p.default_sp, t.total_cost, t.calculated_sp);
            let sp = cap.map_or(scaled, |cap| scaled.min(cap));
            let update = ProductUpdate {
                id: p.id.clone(),
                code: p.code.clone(),
                name: p.name.clone(),
                cost_price: t.total_cost,
                default_sp: sp,
                target_margin: crate::js::dec(if f(sp) > 0.0 { (f(sp) - f(t.total_cost)) / f(sp) } else { 0.0 }),
            };
            // A product priced again keeps its first position, like Map.set.
            match products.iter_mut().find(|u| u.id == update.id) {
                Some(existing) => *existing = update,
                None => products.push(update),
            }
        }

        // The sheet records the price its own product now sells at.
        let own_sp =
            sheet.product_id.as_deref().and_then(|id| products.iter().find(|u| u.id == id)).map(|u| u.default_sp);
        sheets.push(SheetUpdate {
            id: sheet.id.clone(),
            product_name: sheet.product_name.clone(),
            material_total: t.material_total,
            hardware_total: t.hardware_total,
            finishing_total: t.finishing_total,
            machine_total: t.machine_total,
            total_cost: t.total_cost,
            calculated_sp: own_sp.unwrap_or(t.calculated_sp),
        });
    }

    RepricePlan { lines: lines.into_iter().filter(|(_, c)| *c).map(|(l, _)| l).collect(), sheets, products }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn product(id: &str, code: &str, name: &str) -> PricedProduct {
        PricedProduct {
            id: id.into(),
            code: code.into(),
            name: name.into(),
            cost_price: dec!(30),
            default_sp: dec!(60),
        }
    }

    #[test]
    fn rerates_lines_keeps_margin_and_prices_uncosted_siblings() {
        let line = ProductCostLine {
            id: Some("l1".into()),
            sheet_id: Some("s1".into()),
            stage_code: "misc".into(),
            category_name: "C".into(),
            item_name: "Old".into(),
            unit: "pcs".into(),
            qty: Some(dec!(2)),
            rate: dec!(10),
            cost_variety_id: Some("v1".into()),
            line_total: dec!(20),
            ..Default::default()
        };
        let untouched = ProductCostLine {
            id: Some("l2".into()),
            cost_variety_id: Some("v9".into()),
            rate: dec!(5),
            line_total: dec!(10),
            ..line.clone()
        };
        let sheet = SheetRef {
            id: "s1".into(),
            product_id: Some("p1".into()),
            product_code: Some("LC/0001/WL".into()),
            product_name: "Box".into(),
            ..Default::default()
        };
        let rates = [MasterRate {
            id: "v1".into(),
            name: "New".into(),
            default_rate: dec!(20),
            unit: "pcs".into(),
            default_wastage_pct: dec!(10),
            subcategory_name: Some("Sub".into()),
            category_name: None,
        }];
        let products = [
            product("p1", "LC/0001/WL", "Box"),
            product("p2", "LC/0001/BL", "Box"),
            product("p3", "LC/0001/NT", "Box"),
            product("p4", "LC/0002/WL", "Other"),
        ];
        let costed_alone = HashSet::from(["p1".to_string(), "p3".to_string()]);
        let plan = plan_reprice(&RepriceInput {
            rates: &rates,
            lines: &[line, untouched],
            sheets: &[sheet],
            products: &products,
            costed_alone: &costed_alone,
            gst_rate: dec!(0),
        });

        assert_eq!(plan.lines.len(), 1);
        assert_eq!(plan.lines[0].rate, dec!(20));
        assert_eq!(plan.lines[0].item_name, "Sub New");
        assert_eq!(plan.lines[0].wastage_pct, dec!(10));
        let mut ids: Vec<_> = plan.products.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["p1", "p2"]);
        let p1 = plan.products.iter().find(|p| p.id == "p1");
        assert_eq!(p1.map(|p| (p.cost_price, p.default_sp)), Some((dec!(54), dec!(110))));
        assert_eq!(plan.sheets[0].calculated_sp, dec!(110));
    }

    #[test]
    fn never_scales_past_the_mrp_limit() {
        let line = ProductCostLine {
            id: Some("l1".into()),
            sheet_id: Some("s1".into()),
            stage_code: "bought_out".into(),
            unit: "piece".into(),
            rate: dec!(300),
            qty: Some(dec!(1)),
            mrp: Some(dec!(499)),
            cost_variety_id: Some("v1".into()),
            line_total: dec!(300),
            ..Default::default()
        };
        let sheet = SheetRef {
            id: "s1".into(),
            product_id: Some("p1".into()),
            product_code: Some("BO/0001/WL".into()),
            ..Default::default()
        };
        let rates = [MasterRate {
            id: "v1".into(),
            name: "500ml".into(),
            default_rate: dec!(330),
            unit: "piece".into(),
            ..Default::default()
        }];
        let products = [PricedProduct {
            id: "p1".into(),
            code: "BO/0001/WL".into(),
            name: "Bottle".into(),
            cost_price: dec!(300),
            default_sp: dec!(420),
        }];
        let alone = HashSet::from(["p1".to_string()]);
        let plan = plan_reprice(&RepriceInput {
            rates: &rates,
            lines: &[line],
            sheets: &[sheet],
            products: &products,
            costed_alone: &alone,
            gst_rate: dec!(0.18),
        });
        // 420 × 330/300 = 462 would round to 470; MRP 499 caps it at 422.
        assert_eq!(plan.products[0].default_sp, dec!(422));
        assert_eq!(plan.sheets[0].calculated_sp, dec!(422));
    }
}
