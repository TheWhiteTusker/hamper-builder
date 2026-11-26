//! Product costing: line cost, stage totals, SP (port of `costing.ts`).
//! Arithmetic follows JS doubles in the old app's operation order, so a line
//! costs here exactly what the old app saved for it.

use crate::dimensions::{AREA_UNITS, DimUnit, LINEAR_UNITS, dimension_area_f};
use crate::js::{self, dec, f, fo};
use crate::lenient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Overhead % per stage code, e.g. `{ "material": 10 }`, added on top of that stage's lines.
pub type StageOverheads = BTreeMap<String, Decimal>;

/// One row of `product_cost_lines`. Every field defaults, so a half-filled
/// line from the calculator (the TS `Partial<ProductCostLine>`) is valid.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductCostLine {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_id: Option<String>,
    /// material | hardware | finishing | machine | miscellaneous | bought_out
    #[serde(deserialize_with = "lenient::null_default")]
    pub stage_code: String,
    #[serde(deserialize_with = "lenient::null_default")]
    pub category_name: String,
    pub subcategory_name: Option<String>,
    pub variety_name: Option<String>,
    #[serde(deserialize_with = "lenient::null_default")]
    pub item_name: String,
    pub cost_item_id: Option<String>,
    pub cost_variety_id: Option<String>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub length: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub breadth: Option<Decimal>,
    pub dimension_unit: Option<String>,
    #[serde(deserialize_with = "lenient::null_default")]
    pub unit: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub rate: Decimal,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub duration_minutes: Option<Decimal>,
    /// Blank means 1; an explicit 0 means the line costs nothing.
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub qty: Option<Decimal>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub wastage_pct: Decimal,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub calculated_area: Option<Decimal>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub line_total: Decimal,
    /// MRP printed on a bought-out item, per piece and GST included.
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub mrp: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}

impl ProductCostLine {
    /// The line's quantity: blank is 1.
    pub fn qty_or_one(&self) -> Decimal {
        self.qty.unwrap_or(Decimal::ONE)
    }

    pub(crate) fn qty_f(&self) -> f64 {
        self.qty.map_or(1.0, f)
    }

    pub fn stage(&self) -> String {
        self.stage_code.to_lowercase()
    }
}

/// Minutes in one unit of a time-quoted rate.
fn minutes_per(unit: &str) -> Option<f64> {
    match unit.trim().to_lowercase().as_str() {
        "min" | "mins" | "minute" | "minutes" => Some(1.0),
        "hour" | "hr" | "hours" => Some(60.0),
        _ => None,
    }
}

/// A rate quoted per minute or per hour: priced on duration, not size.
pub fn is_time_unit(unit: Option<&str>) -> bool {
    minutes_per(unit.unwrap_or_default()).is_some()
}

/// A rate priced on length or area, so the line's dimensions count. Blank means sq ft.
pub fn is_sized_unit(unit: Option<&str>) -> bool {
    let u = match unit {
        Some(u) if !u.is_empty() => u,
        _ => "sq ft",
    };
    let u = u.trim().to_lowercase();
    LINEAR_UNITS.contains(&u.as_str()) || AREA_UNITS.contains(&u.as_str())
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineCost {
    pub calculated_area: Decimal,
    pub line_total: Decimal,
}

/// [`calculate_line_cost`] on doubles: (calculated_area, line_total).
pub(crate) fn line_cost_f(line: &ProductCostLine) -> (f64, f64) {
    let stage = line.stage();
    let rate = f(line.rate);
    let qty = line.qty_f();
    let wastage = f(line.wastage_pct);

    // Machine time: minutes × the rate per minute (or per hour ÷ 60). A machine
    // rate quoted by size (engraving per sq inch) is priced on dimensions below.
    let machine_unit = if line.unit.is_empty() { "min" } else { line.unit.as_str() };
    if stage == "machine"
        && let Some(per) = minutes_per(machine_unit)
    {
        let duration = fo(line.duration_minutes);
        let base = (duration / per) * rate * qty;
        return (duration, js::round2(base * (1.0 + wastage / 100.0)));
    }

    let dim = DimUnit::parse(line.dimension_unit.as_deref().filter(|u| !u.is_empty()));
    let unit = if line.unit.is_empty() { "sq ft" } else { line.unit.as_str() };
    let key = unit.trim().to_lowercase();
    let (l, b) = (fo(line.length), fo(line.breadth));
    let is_linear = LINEAR_UNITS.contains(&key.as_str());
    let is_area = AREA_UNITS.contains(&key.as_str());

    // The exact area is costed; it is rounded only for display.
    let (area, base) = if (is_linear && l > 0.0) || (is_area && l > 0.0 && b > 0.0) {
        let exact = dimension_area_f(l, b, dim, unit);
        (js::round2(exact), exact * rate * qty)
    } else {
        // Per piece, or no dimensions to multiply.
        (1.0, rate * qty)
    };
    (area, js::round2(base * (1.0 + wastage / 100.0)))
}

/// A cost line's priced quantity (area, length or minutes) and its total.
pub fn calculate_line_cost(line: &ProductCostLine) -> LineCost {
    let (area, total) = line_cost_f(line);
    LineCost { calculated_area: dec(area), line_total: dec(total) }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostSheetTotals {
    pub material_total: Decimal,
    pub hardware_total: Decimal,
    pub finishing_total: Decimal,
    pub machine_total: Decimal,
    pub other_total: Decimal,
    pub total_cost: Decimal,
    pub markup_pct: Decimal,
    pub calculated_sp: Decimal,
    pub target_margin: Decimal,
}

/// A stage's overhead multiplier, `1 + pct/100`.
pub(crate) fn overhead_factor(overheads: Option<&StageOverheads>, stage: &str) -> f64 {
    1.0 + fo(overheads.and_then(|o| o.get(stage)).copied()) / 100.0
}

/// A product's selling price after its cost changes, scaled with the cost so
/// its margin holds (including a hand-set price). Without a previous cost and
/// price there is nothing to scale, so `fallback_sp` is used.
pub fn scaled_price(old_cost: Decimal, old_sp: Decimal, new_cost: Decimal, fallback_sp: Decimal) -> Decimal {
    let (cost, sp) = (f(old_cost), f(old_sp));
    dec(if cost > 0.0 && sp > 0.0 { js::ceil10((sp * f(new_cost)) / cost) } else { js::ceil10(f(fallback_sp)) })
}

/// A stage's subtotal with its overhead % added.
pub fn with_overhead(amount: Decimal, overhead_pct: Decimal) -> Decimal {
    dec(js::round2(f(amount) * (1.0 + f(overhead_pct) / 100.0)))
}

fn sp_from_markup_f(total_cost: f64, markup_pct: f64) -> f64 {
    let divisor = 1.0 - markup_pct / 100.0;
    if divisor > 0.0 { js::ceil10(total_cost / divisor) } else { js::ceil10(total_cost) }
}

/// SP from cost and markup: CP ÷ (1 − markup%), rounded up to the next 10. A
/// markup of 100% or more has no finite price, so it falls back to cost.
pub fn sp_from_markup(total_cost: Decimal, markup_pct: Decimal) -> Decimal {
    dec(sp_from_markup_f(f(total_cost), f(markup_pct)))
}

/// Sums the lines by stage with each stage's overhead %, then prices the sheet.
pub fn calculate_cost_sheet_totals(
    lines: &[ProductCostLine],
    markup_pct: Decimal,
    overheads: &StageOverheads,
) -> CostSheetTotals {
    let (mut material, mut hardware, mut finishing, mut machine, mut other) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for line in lines {
        let stage = line.stage();
        // Scaling each line by its stage's overhead equals scaling the stage subtotal.
        let total = line_cost_f(line).1 * overhead_factor(Some(overheads), &stage);
        match stage.as_str() {
            "material" => material += total,
            "hardware" => hardware += total,
            "finishing" => finishing += total,
            "machine" => machine += total,
            // miscellaneous, bought_out, or any stage added later
            _ => other += total,
        }
    }
    let [material, hardware, finishing, machine, other] =
        [material, hardware, finishing, machine, other].map(js::round2);
    let total_cost = js::round2(material + hardware + finishing + machine + other);
    let sp = sp_from_markup_f(total_cost, f(markup_pct));
    let target_margin = if sp > 0.0 { (sp - total_cost) / sp } else { 0.0 };
    CostSheetTotals {
        material_total: dec(material),
        hardware_total: dec(hardware),
        finishing_total: dec(finishing),
        machine_total: dec(machine),
        other_total: dec(other),
        total_cost: dec(total_cost),
        markup_pct,
        calculated_sp: dec(sp),
        target_margin: dec(js::round2(target_margin)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn line(stage: &str, unit: &str, rate: Decimal, qty: Decimal, wastage: Decimal) -> ProductCostLine {
        ProductCostLine {
            stage_code: stage.into(),
            unit: unit.into(),
            rate,
            qty: Some(qty),
            wastage_pct: wastage,
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)] // mirrors the TS fixtures' field list
    fn sized(
        stage: &str,
        l: Decimal,
        b: Decimal,
        dim: &str,
        unit: &str,
        rate: Decimal,
        qty: Decimal,
        w: Decimal,
    ) -> ProductCostLine {
        ProductCostLine {
            length: Some(l),
            breadth: Some(b),
            dimension_unit: Some(dim.into()),
            ..line(stage, unit, rate, qty, w)
        }
    }

    fn machine(minutes: Decimal, unit: &str, rate: Decimal, w: Decimal) -> ProductCostLine {
        ProductCostLine { duration_minutes: Some(minutes), ..line("machine", unit, rate, dec!(1), w) }
    }

    #[test]
    fn dimensional_material() {
        let c =
            calculate_line_cost(&sized("material", dec!(12), dec!(12), "inch", "sq ft", dec!(85), dec!(2), dec!(10)));
        assert_eq!(c.calculated_area, dec!(1));
        assert_eq!(c.line_total, dec!(187));
    }

    #[test]
    fn machine_per_minute() {
        let c = calculate_line_cost(&machine(dec!(15), "", dec!(12), dec!(5)));
        assert_eq!(c.calculated_area, dec!(15));
        assert_eq!(c.line_total, dec!(189));
    }

    #[test]
    fn linear_feet_material() {
        let c = calculate_line_cost(&sized("material", dec!(36), dec!(0), "inch", "feet", dec!(20), dec!(2), dec!(0)));
        assert_eq!(c.calculated_area, dec!(3));
        assert_eq!(c.line_total, dec!(120));
    }

    #[test]
    fn sq_cm_finishing() {
        let c =
            calculate_line_cost(&sized("finishing", dec!(10), dec!(20), "cm", "sq cm", dec!(0.5), dec!(1), dec!(0)));
        assert_eq!(c.calculated_area, dec!(200));
        assert_eq!(c.line_total, dec!(100));
    }

    #[test]
    fn sized_units_count_dimensions_and_pieces_do_not() {
        for u in [Some("sqft"), Some("sq inch"), Some("sqinch"), Some("meter"), Some("running ft"), Some(""), None] {
            assert!(is_sized_unit(u), "{u:?}");
        }
        for u in ["piece", "set", "kg", "min"] {
            assert!(!is_sized_unit(Some(u)), "{u}");
        }
        let l = sized("material", dec!(3), dec!(4), "", "sqinch", dec!(2), dec!(1), dec!(0));
        assert_eq!(calculate_line_cost(&ProductCostLine { qty: Some(dec!(1)), ..l }).line_total, dec!(24));
    }

    #[test]
    fn sheet_totals_aggregate_all_stages_and_apply_markup() {
        let lines = [
            sized("material", dec!(12), dec!(12), "inch", "sq ft", dec!(85), dec!(2), dec!(10)),
            line("hardware", "piece", dec!(10), dec!(4), dec!(0)),
            sized("finishing", dec!(12), dec!(12), "inch", "sq ft", dec!(35), dec!(1), dec!(0)),
            machine(dec!(15), "", dec!(12), dec!(5)),
        ];
        let t = calculate_cost_sheet_totals(&lines, dec!(50), &StageOverheads::new());
        assert_eq!(t.material_total, dec!(187));
        assert_eq!(t.hardware_total, dec!(40));
        assert_eq!(t.finishing_total, dec!(35));
        assert_eq!(t.machine_total, dec!(189));
        assert_eq!(t.total_cost, dec!(451));
        assert_eq!(t.markup_pct, dec!(50));
        assert_eq!(t.calculated_sp, dec!(910));
        assert_eq!(t.target_margin, dec!(0.5));
    }

    #[test]
    fn misc_and_bought_out_roll_into_total_cost() {
        let lines = [
            line("material", "piece", dec!(100), dec!(1), dec!(0)),
            line("miscellaneous", "piece", dec!(50), dec!(2), dec!(0)),
            line("bought_out", "piece", dec!(200), dec!(1), dec!(25)),
            line("hardware", "piece", dec!(300), dec!(0), dec!(0)),
        ];
        let t = calculate_cost_sheet_totals(&lines, dec!(50), &StageOverheads::new());
        assert_eq!(t.material_total, dec!(100));
        assert_eq!(t.hardware_total, dec!(0));
        assert_eq!(t.other_total, dec!(350), "bought-out 200 + 25% wastage");
        assert_eq!(t.total_cost, dec!(450));
        assert_eq!(t.calculated_sp, dec!(900));
    }

    #[test]
    fn sp_from_markup_and_100_percent_falls_back_to_cost() {
        let lines = [line("material", "piece", dec!(600), dec!(1), dec!(0))];
        let none = StageOverheads::new();
        let forty = calculate_cost_sheet_totals(&lines, dec!(40), &none);
        assert_eq!(forty.total_cost, dec!(600));
        assert_eq!(forty.calculated_sp, dec!(1000));
        assert_eq!(forty.target_margin, dec!(0.4));
        assert_eq!(calculate_cost_sheet_totals(&lines, dec!(0), &none).calculated_sp, dec!(600));
        for bad in [dec!(100), dec!(150)] {
            assert_eq!(calculate_cost_sheet_totals(&lines, bad, &none).calculated_sp, dec!(600));
        }
    }

    #[test]
    fn stage_overhead_applies_to_its_stage_only() {
        let lines = [
            line("material", "piece", dec!(100), dec!(2), dec!(0)),
            line("hardware", "piece", dec!(50), dec!(1), dec!(0)),
            line("miscellaneous", "piece", dec!(40), dec!(1), dec!(0)),
        ];
        let oh = StageOverheads::from([("material".into(), dec!(10)), ("miscellaneous".into(), dec!(25))]);
        let t = calculate_cost_sheet_totals(&lines, dec!(50), &oh);
        assert_eq!(t.material_total, dec!(220));
        assert_eq!(t.hardware_total, dec!(50));
        assert_eq!(t.other_total, dec!(50));
        assert_eq!(t.total_cost, dec!(320));
        assert_eq!(t.calculated_sp, dec!(640));
        let blank = StageOverheads::from([("material".into(), dec!(0))]);
        assert_eq!(calculate_cost_sheet_totals(&lines, dec!(50), &blank).total_cost, dec!(290));
        assert_eq!(with_overhead(dec!(200), dec!(10)), dec!(220));
        assert_eq!(with_overhead(dec!(99.99), dec!(0)), dec!(99.99));
    }

    #[test]
    fn machine_rates_follow_their_unit() {
        assert_eq!(calculate_line_cost(&machine(dec!(30), "min", dec!(3), dec!(0))).line_total, dec!(90));
        assert_eq!(calculate_line_cost(&machine(dec!(30), "", dec!(3), dec!(0))).line_total, dec!(90));
        assert_eq!(calculate_line_cost(&machine(dec!(30), "hour", dec!(600), dec!(0))).line_total, dec!(300));
        let engraving = ProductCostLine {
            length: Some(dec!(4)),
            breadth: Some(dec!(5)),
            dimension_unit: Some("inch".into()),
            ..machine(dec!(30), "sq inch", dec!(0.25), dec!(0))
        };
        let c = calculate_line_cost(&engraving);
        assert_eq!(c.calculated_area, dec!(20));
        assert_eq!(c.line_total, dec!(5));
    }

    #[test]
    fn exact_area_is_costed_and_rounded_for_display() {
        let c =
            calculate_line_cost(&sized("material", dec!(5), dec!(5), "inch", "sq ft", dec!(1000), dec!(1), dec!(0)));
        assert_eq!(c.calculated_area, dec!(0.17));
        assert_eq!(c.line_total, dec!(173.61));
    }

    #[test]
    fn repriced_product_keeps_its_margin() {
        assert_eq!(scaled_price(dec!(55.5), dec!(277.5), dec!(61.05), dec!(999)), dec!(310));
        assert_eq!(scaled_price(dec!(321.78), dec!(2883.3), dec!(353.96), dec!(999)), dec!(3180));
        assert_eq!(scaled_price(dec!(0), dec!(0), dec!(100), dec!(500)), dec!(500));
    }

    /// costing-e2e.test.ts: birch, magnets, PU polish and laser cutting at 55%.
    #[test]
    fn full_workflow_birch_box() {
        let lines = [
            sized("material", dec!(10), dec!(8), "inch", "sq ft", dec!(85), dec!(2), dec!(10)),
            sized("material", dec!(10), dec!(8), "inch", "sq ft", dec!(55), dec!(1), dec!(10)),
            line("hardware", "piece", dec!(10), dec!(2), dec!(0)),
            sized("finishing", dec!(18), dec!(12), "inch", "sq ft", dec!(35), dec!(1), dec!(5)),
            machine(dec!(12), "min", dec!(12), dec!(5)),
        ];
        let costs: Vec<_> = lines.iter().map(calculate_line_cost).collect();
        assert_eq!((costs[0].calculated_area, costs[0].line_total), (dec!(0.56), dec!(103.89)));
        assert_eq!((costs[1].calculated_area, costs[1].line_total), (dec!(0.56), dec!(33.61)));
        assert_eq!(costs[2].line_total, dec!(20));
        assert_eq!((costs[3].calculated_area, costs[3].line_total), (dec!(1.5), dec!(55.13)));
        assert_eq!((costs[4].calculated_area, costs[4].line_total), (dec!(12), dec!(151.2)));

        let t = calculate_cost_sheet_totals(&lines, dec!(55), &StageOverheads::new());
        assert_eq!(t.material_total, dec!(137.5));
        assert_eq!(t.hardware_total, dec!(20));
        assert_eq!(t.finishing_total, dec!(55.13));
        assert_eq!(t.machine_total, dec!(151.2));
        assert_eq!(t.total_cost, dec!(363.83));
        assert_eq!(t.calculated_sp, dec!(810));
        assert_eq!(t.target_margin, dec!(0.55));
    }
}
