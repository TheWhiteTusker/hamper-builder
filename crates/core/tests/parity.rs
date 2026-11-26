//! Replays `scripts/fixtures/parity.json` (the old app's TypeScript run on
//! thousands of generated inputs) against the Rust port.
//!
//! Every figure must match exactly, unrounded sums and margins included: the
//! port does the old app's double arithmetic, so the two agree bit for bit.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use lattice_core::costing::{
    ProductCostLine, StageOverheads, calculate_cost_sheet_totals, calculate_line_cost, scaled_price,
};
use lattice_core::money::amount_in_words;
use lattice_core::mrp::{mrp_limit, round_sp};
use lattice_core::numbers::round2;
use lattice_core::pricing::{
    HamperLineInput, HamperOpts, QuoteLineInput, QuoteOpts, margin_pct_of, price_hamper, price_quote, sp_at_margin,
};
use lattice_core::product_code::{format_product_code, parse_product_code};
use lattice_core::{Decimal, colors};
use serde_json::Value;
use std::str::FromStr;

fn d(v: &Value) -> Decimal {
    match v {
        Value::Number(n) => Decimal::from_str(&n.as_f64().unwrap().to_string())
            .or_else(|_| Decimal::from_scientific(&n.to_string()))
            .unwrap(),
        Value::Null => Decimal::ZERO,
        other => panic!("not a number: {other}"),
    }
}

fn od(v: &Value) -> Option<Decimal> {
    (!v.is_null()).then(|| d(v))
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

#[derive(Default)]
struct Report {
    checked: usize,
    failures: Vec<String>,
}

impl Report {
    fn exact(&mut self, what: &str, i: usize, ts: Option<Decimal>, rs: Option<Decimal>) {
        self.checked += 1;
        if ts.map(|v| v.normalize()) != rs.map(|v| v.normalize()) {
            self.failures.push(format!("{what} #{i}: ts {ts:?} rs {rs:?}"));
        }
    }

    fn text(&mut self, what: &str, i: usize, ts: &str, rs: &str) {
        self.checked += 1;
        if ts != rs {
            self.failures.push(format!("{what} #{i}: ts {ts:?} rs {rs:?}"));
        }
    }
}

fn cost_line(v: &Value) -> ProductCostLine {
    let text = |k: &str| v[k].as_str().map(str::to_string);
    ProductCostLine {
        stage_code: text("stage_code").unwrap_or_default(),
        unit: text("unit").unwrap_or_default(),
        rate: d(&v["rate"]),
        qty: od(&v["qty"]),
        wastage_pct: d(&v["wastage_pct"]),
        length: od(&v["length"]),
        breadth: od(&v["breadth"]),
        dimension_unit: text("dimension_unit"),
        duration_minutes: od(&v["duration_minutes"]),
        mrp: od(&v.get("mrp").cloned().unwrap_or(Value::Null)),
        ..Default::default()
    }
}

fn overheads(v: &Value) -> StageOverheads {
    v.as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), d(v))).collect()).unwrap_or_default()
}

#[test]
fn rust_port_matches_the_typescript() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/fixtures/parity.json");
    let raw = std::fs::read_to_string(path).expect("run `node scripts/gen-fixtures.mjs` first");
    let f: Value = serde_json::from_str(&raw).unwrap();
    let mut r = Report::default();

    for (i, c) in f["hamper"].as_array().unwrap().iter().enumerate() {
        let lines: Vec<HamperLineInput> = c["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| HamperLineInput {
                qty: d(&l["qty"]),
                unit_cp: d(&l["unitCp"]),
                unit_sp: d(&l["unitSp"]),
                counts_as_item: l["countsAsItem"].as_bool().unwrap_or(true),
            })
            .collect();
        let o = &c["opts"];
        let opts = HamperOpts {
            discount_pct: d(&o["discountPct"]),
            target_sp: od(&o["targetSp"]),
            final_catalogue_sp: od(&o["finalCatalogueSp"]),
        };
        let p = price_hamper(&lines, &opts);
        let out = &c["out"];
        r.exact("hamper.total_cp", i, od(&out["totalCp"]), Some(p.total_cp));
        r.exact("hamper.base_sp", i, od(&out["baseSp"]), Some(p.base_sp));
        r.exact("hamper.items", i, od(&out["numberOfItems"]), Some(p.number_of_items));
        r.exact("hamper.after_discount", i, od(&out["spAfterDiscount"]), Some(p.sp_after_discount));
        r.exact("hamper.variance", i, od(&out["variance"]), p.variance);
        r.exact("hamper.gross_profit", i, od(&out["grossProfit"]), p.gross_profit);
        r.exact("hamper.final_margin", i, od(&out["finalMargin"]), p.final_margin);
    }

    for (i, c) in f["quote"].as_array().unwrap().iter().enumerate() {
        let lines: Vec<QuoteLineInput> = c["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| QuoteLineInput {
                qty: d(&l["qty"]),
                catalogue_price: d(&l["cataloguePrice"]),
                discount_pct: d(&l["discountPct"]),
            })
            .collect();
        let o = &c["opts"];
        let opts = QuoteOpts {
            quote_structure: s(&o["quoteStructure"]),
            order_discount: d(&o["orderDiscount"]),
            adj1: d(&o["adj1"]),
            adj2: d(&o["adj2"]),
            gst_rate: d(&o["gstRate"]),
        };
        let q = price_quote(&lines, &opts);
        let out = &c["out"];
        r.exact("quote.subtotal", i, od(&out["subtotal"]), Some(q.subtotal));
        r.exact("quote.total_qty", i, od(&out["totalQty"]), Some(q.total_qty));
        r.exact("quote.taxable", i, od(&out["taxableValue"]), q.taxable_value);
        r.exact("quote.gst", i, od(&out["gstAmount"]), q.gst_amount);
        r.exact("quote.grand_total", i, od(&out["grandTotal"]), q.grand_total);
    }

    for (i, c) in f["line"].as_array().unwrap().iter().enumerate() {
        let got = calculate_line_cost(&cost_line(&c["line"]));
        r.exact("line.area", i, od(&c["out"]["calculated_area"]).map(round2), Some(round2(got.calculated_area)));
        r.exact("line.total", i, od(&c["out"]["line_total"]), Some(got.line_total));
    }

    for (i, c) in f["sheet"].as_array().unwrap().iter().enumerate() {
        let lines: Vec<ProductCostLine> = c["lines"].as_array().unwrap().iter().map(cost_line).collect();
        let t = calculate_cost_sheet_totals(&lines, d(&c["markup"]), &overheads(&c["overheads"]));
        let out = &c["out"];
        for (k, v) in [
            ("material_total", t.material_total),
            ("hardware_total", t.hardware_total),
            ("finishing_total", t.finishing_total),
            ("machine_total", t.machine_total),
            ("other_total", t.other_total),
            ("total_cost", t.total_cost),
            ("calculated_sp", t.calculated_sp),
            ("target_margin", t.target_margin),
        ] {
            r.exact(&format!("sheet.{k}"), i, od(&out[k]), Some(v));
        }
    }

    for (i, c) in f["mrp"].as_array().unwrap().iter().enumerate() {
        let lines: Vec<ProductCostLine> = c["lines"].as_array().unwrap().iter().map(cost_line).collect();
        let oh = overheads(&c["overheads"]);
        let got = mrp_limit(&lines, Some(&oh), d(&c["total_cost"]), d(&c["gst"]));
        r.exact("mrp.limit", i, od(&c["out"]), got);
    }

    for (i, c) in f["round_sp"].as_array().unwrap().iter().enumerate() {
        r.exact("round_sp", i, od(&c["out"]), Some(round_sp(d(&c["sp"]), od(&c["limit"]))));
    }
    for (i, c) in f["scaled"].as_array().unwrap().iter().enumerate() {
        let got = scaled_price(d(&c["old_cost"]), d(&c["old_sp"]), d(&c["new_cost"]), d(&c["fallback"]));
        r.exact("scaled_price", i, od(&c["out"]), Some(got));
    }
    for (i, c) in f["margin"].as_array().unwrap().iter().enumerate() {
        r.exact("sp_at_margin", i, od(&c["sp_at"]), sp_at_margin(d(&c["cp"]), d(&c["margin"])));
        r.exact("margin_pct_of", i, od(&c["pct_of"]), margin_pct_of(d(&c["cp"]), d(&c["sp"])));
    }
    for (i, c) in f["words"].as_array().unwrap().iter().enumerate() {
        r.text("amount_in_words", i, &s(&c["out"]), &amount_in_words(d(&c["amount"])));
    }

    for (i, c) in f["code"].as_array().unwrap().iter().enumerate() {
        let p = parse_product_code(&s(&c["raw"]));
        let ts = &c["parsed"];
        r.text("code.category", i, &s(&ts["categoryCode"]), &p.category_code);
        r.text("code.serial", i, &s(&ts["serial"]), &p.serial);
        r.text("code.color_code", i, &s(&ts["colorCode"]), &p.color_code);
        r.text("code.color_name", i, &s(&ts["colorName"]), &p.color_name);
        r.text("code.valid", i, &ts["isValid"].to_string(), &p.is_valid.to_string());
        let formatted = format_product_code(&s(&c["cat"]), &s(&c["serial"]), &s(&c["color"]));
        r.text("code.format", i, &s(&c["formatted"]), &formatted);
        if let Some(want) = c["color_code"].as_str() {
            r.text("code.color", i, want, &colors::color_code(&s(&c["color"])));
        }
        if let Some(want) = c["derived"].as_str() {
            r.text("code.derive", i, want, &colors::derive_category_code(&s(&c["cat"])));
        }
    }

    assert!(
        r.failures.is_empty(),
        "{} of {} checks differ from the TypeScript:\n{}",
        r.failures.len(),
        r.checked,
        r.failures.iter().take(25).map(String::as_str).collect::<Vec<_>>().join("\n")
    );
    eprintln!("parity: {} checks, all match", r.checked);
}
