//! Product Master as an editable workbook, and reading its costing back (port
//! of `product-sheet.ts`, `product-sheet-lines.ts`, `product-sheet-costing.ts`).
//!
//! Tab 1 is one row per product in the column order `build_import` reads, then
//! per stage its lines' total and an overhead %. Tab 2 (Cost Lines) holds every
//! product's lines, tab 3 the Rates. Formulas are written only where they give
//! back today's figure, so a download-and-upload never moves a cost or price.

use super::{Cached, Cell, Column, Sheet, formula_row, js_num, letter};
use crate::cost_tree::{CostRows, CostVariety, StageNode, build_cost_tree};
use crate::costing::{
    CostSheetTotals, ProductCostLine, StageOverheads, calculate_cost_sheet_totals, calculate_line_cost,
};
use crate::import::{Grid, cell, numbered};
use crate::js::{self, dec, f, fo};
use crate::numbers::{normalize_pct_str, parse_num};
use crate::product_code::parse_product_code;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const SHEET_NAME: &str = "Product Master";
pub const LINES_SHEET: &str = "Cost Lines";
pub const RATES_SHEET: &str = "Rates";

/// (code, label) per stage, in Product Master column order.
pub const SHEET_STAGES: [(&str, &str); 6] = [
    ("material", "Material"),
    ("hardware", "Hardware"),
    ("finishing", "Finishing"),
    ("machine", "Machine"),
    ("miscellaneous", "Misc"),
    ("bought_out", "Bought Out"),
];

/// Product Master column I: each stage takes two columns, its lines' total then its overhead %.
pub const STAGE_COL: usize = 8;

/// Cost Lines columns, in order.
pub const LINE_HEADERS: [&str; 16] = [
    "Product Code",
    "Product Name",
    "Type",
    "Category",
    "Subcategory",
    "Variety",
    "Length",
    "Breadth",
    "Size In",
    "Qty",
    "Minutes",
    "Unit",
    "Rate",
    "Wastage %",
    "MRP",
    "Line Total",
];

/// The 0-based Cost Lines column of a header.
pub fn line_col(header: &str) -> usize {
    LINE_HEADERS.iter().position(|h| *h == header).unwrap_or(0)
}

/// The sheet's spelling of a stage: its label, else the given name.
pub fn stage_label<'a>(code: &str, name: &'a str) -> &'a str {
    SHEET_STAGES.iter().find(|(c, _)| *c == code).map_or(name, |(_, label)| label)
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

// ------------------------------------------------------------ formulas

fn at(header: &str, r: usize) -> String {
    format!("{}{r}", letter(line_col(header)))
}

const TIME: &str = r#""min",1;"mins",1;"minute",1;"minutes",1;"hour",60;"hr",60;"hours",60"#;
const LINEAR: &str = r#""feet",0.08333333333333333;"ft",0.08333333333333333;"running ft",0.08333333333333333;"rft",0.08333333333333333;"inch",1;"in",1;"mm",25.4;"cm",2.54;"meter",0.0254;"m",0.0254"#;
const AREA: &str = r#""sq ft",0.006944444444444444;"sqft",0.006944444444444444;"sft",0.006944444444444444;"sq inch",1;"sqinch",1;"sq in",1;"sqin",1;"sq mm",645.16;"sqmm",645.16;"sq cm",6.4516;"sqcm",6.4516"#;

fn lookup_key(r: usize) -> String {
    ["Type", "Category", "Subcategory", "Variety"].map(|h| format!("TRIM({})", at(h, r))).join(r#"&"|"&"#)
}

/// Unit (E), Rate (F) or Wastage % (G) of the line's item on the Rates tab; blank when it isn't there.
fn lookup(col: char, r: usize) -> String {
    format!(
        r#"IF({code}="","",IFERROR(INDEX({RATES_SHEET}!${col}:${col},MATCH({key},{RATES_SHEET}!$H:$H,0)),""))"#,
        code = at("Product Code", r),
        key = lookup_key(r)
    )
}

/// Line Total worked out the way `calculate_line_cost` does, so Excel previews costs as you type.
fn line_total(r: usize) -> String {
    let ty = format!("LOWER(TRIM({}))", at("Type", r));
    let machine = format!("LEFT({ty},7)=\"machine\"");
    let unit = at("Unit", r);
    let u = format!(r#"IF(TRIM({unit})="",IF({machine},"min","sq ft"),LOWER(TRIM({unit})))"#);
    let factor = |rows: &str| format!("IFERROR(VLOOKUP({u},{{{rows}}},2,FALSE),0)");
    let size = at("Size In", r);
    let inches = |h: &str| format!(r#"N({})/IF({size}="mm",25.4,IF({size}="cm",2.54,1))"#, at(h, r));
    let qty = at("Qty", r);
    let each = format!(r#"N({})*IF({qty}="",1,N({qty}))"#, at("Rate", r));
    let (len, wide) = (format!("N({})>0", at("Length", r)), format!("N({})>0", at("Breadth", r)));
    let base = format!(
        "IF(AND({machine},{t}>0),N({min})/{t}*{each},IF(AND({l}>0,{len}),{il}*{l}*{each},IF(AND({a}>0,{len},{wide}),{il}*{ib}*{a}*{each},{each})))",
        t = factor(TIME),
        l = factor(LINEAR),
        a = factor(AREA),
        min = at("Minutes", r),
        il = inches("Length"),
        ib = inches("Breadth"),
    );
    let wastage = format!(r#"IF(LEFT({ty},6)="bought",0,N({})/100)"#, at("Wastage %", r));
    format!(r#"IF({}="","",ROUND({base}*(1+{wastage}),2))"#, at("Product Code", r))
}

/// The product's name, from its row on the Product Master tab.
fn product_name_formula(r: usize) -> String {
    let code = at("Product Code", r);
    format!(r#"IF({code}="","",IFERROR(INDEX('{SHEET_NAME}'!$C:$C,MATCH({code},'{SHEET_NAME}'!$A:$A,0)),""))"#)
}

fn line_columns() -> Vec<Column> {
    LINE_HEADERS
        .iter()
        .map(|&h| {
            let width = match h {
                "Product Code" => 16,
                "Product Name" => 30,
                "Category" | "Subcategory" => 18,
                "Variety" => 20,
                _ => 10,
            };
            let col = Column::new(h, width);
            let col = if matches!(h, "Rate" | "MRP" | "Line Total") { col.money() } else { col };
            match h {
                "Product Name" => col.calc(product_name_formula),
                "Unit" => col.calc(|r| lookup('E', r)),
                "Rate" => col.calc(|r| lookup('F', r)),
                "Wastage %" => col.calc(|r| lookup('G', r)),
                "Line Total" => col.calc(line_total),
                _ => col,
            }
        })
        .collect()
}

fn rate_columns() -> Vec<Column> {
    vec![
        Column::new("Type", 12),
        Column::new("Category", 20),
        Column::new("Subcategory", 20),
        Column::new("Variety", 22),
        Column::new("Unit", 10),
        Column::new("Rate", 11).money(),
        Column::new("Wastage %", 11),
        Column::new("Lookup key", 40).calc(|r| format!(r#"TRIM(A{r})&"|"&TRIM(B{r})&"|"&TRIM(C{r})&"|"&TRIM(D{r})"#)),
    ]
}

// ------------------------------------------------------ rates and lines tabs

struct Item<'a> {
    ty: String,
    category: &'a str,
    subcategory: &'a str,
    v: &'a CostVariety,
}

struct Master<'a> {
    items: Vec<Item<'a>>,
    types: HashMap<String, String>,
}

/// Every variety in the master, with its Type as the sheet spells it.
fn master_items<'a>(tree: &'a [StageNode]) -> Master<'a> {
    let mut items = Vec::new();
    let mut types = HashMap::new();
    for stage in tree {
        let ty = stage_label(&stage.row.code, &stage.row.name).to_string();
        types.insert(stage.row.code.clone(), ty.clone());
        for c in &stage.categories {
            for s in &c.subcategories {
                for v in &s.varieties {
                    items.push(Item { ty: ty.clone(), category: &c.row.name, subcategory: &s.row.name, v });
                }
            }
        }
    }
    Master { items, types }
}

/// A blank row before each new subcategory. Reference only, so no formulas there.
pub fn rates_sheet(rows: &CostRows) -> Sheet {
    let tree = build_cost_tree(rows);
    let cols = rate_columns();
    let mut out: Vec<Vec<Cell>> = Vec::new();
    let mut last = String::new();
    for Item { ty, category, subcategory, v } in master_items(&tree).items {
        let sub = format!("{ty}|{category}|{subcategory}");
        if !out.is_empty() && sub != last {
            out.push(Vec::new());
        }
        last = sub;
        let key = [ty.as_str(), category, subcategory, &v.name].map(str::trim).join("|");
        let r = out.len() + 2;
        out.push(vec![
            Cell::text(&ty),
            Cell::text(category),
            Cell::text(subcategory),
            Cell::text(&v.name),
            Cell::text(&v.unit),
            v.default_rate.into(),
            v.default_wastage_pct.into(),
            Cell::formula(cols.get(7).and_then(|c| c.formula_at(r)).unwrap_or_default(), Cached::Text(key)),
        ]);
    }
    Sheet { name: RATES_SHEET.into(), columns: cols, rows: out, blank_rows: 0 }
}

pub struct ProductLines<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub lines: &'a [ProductCostLine],
}

/// Line totals per stage code, per product code.
pub type StageSums = HashMap<String, BTreeMap<String, f64>>;

/// Every product's lines, brought up to the master's current rates as the
/// calculator shows them, and each product's line total per stage.
pub fn lines_sheet(products: &[ProductLines], rows: &CostRows) -> (Sheet, StageSums) {
    let tree = build_cost_tree(rows);
    let master = master_items(&tree);
    let by_id: HashMap<&str, &Item> = master.items.iter().map(|i| (i.v.id.as_str(), i)).collect();
    let cols = line_columns();
    let mut out: Vec<Vec<Cell>> = Vec::new();
    let mut sums = StageSums::new();

    for p in products {
        let mut sum: BTreeMap<String, f64> = BTreeMap::new();
        // A blank row between products, carrying the formulas so it can be typed into.
        if !out.is_empty() && !p.lines.is_empty() {
            out.push(formula_row(&cols, out.len() + 2));
        }
        let mut saved: Vec<&ProductCostLine> = p.lines.iter().collect();
        saved.sort_by_key(|l| l.sort_order.unwrap_or(0));
        for saved in saved {
            let m = saved.cost_variety_id.as_deref().and_then(|id| by_id.get(id));
            let l = match m {
                Some(m) => ProductCostLine {
                    category_name: m.category.to_string(),
                    subcategory_name: Some(m.subcategory.to_string()),
                    variety_name: Some(m.v.name.clone()),
                    rate: m.v.default_rate,
                    unit: m.v.unit.clone(),
                    wastage_pct: if saved.stage_code == "bought_out" { Decimal::ZERO } else { m.v.default_wastage_pct },
                    ..saved.clone()
                },
                None => saved.clone(),
            };
            let total = calculate_line_cost(&l).line_total;
            *sum.entry(l.stage_code.clone()).or_default() += f(total);
            let n = out.len() + 2;
            let from_rates = |col: char, v: Cell| match (m, v) {
                (Some(_), Cell::Num(x)) => Cell::formula(lookup(col, n), Cached::Num(x)),
                (Some(_), Cell::Text(x)) => Cell::formula(lookup(col, n), Cached::Text(x)),
                (_, v) => v,
            };
            let ty = master
                .types
                .get(&l.stage_code)
                .cloned()
                .unwrap_or_else(|| stage_label(&l.stage_code, &l.stage_code).into());
            let opt_num = |v: Option<Decimal>| v.map_or(Cell::Empty, Cell::Num);
            out.push(vec![
                Cell::text(p.code),
                Cell::formula(product_name_formula(n), Cached::Text(p.name.to_string())),
                Cell::text(ty),
                Cell::text(&l.category_name),
                Cell::text(l.subcategory_name.clone().unwrap_or_default()),
                Cell::text(l.variety_name.clone().unwrap_or_default()),
                opt_num(l.length),
                opt_num(l.breadth),
                Cell::text(l.dimension_unit.clone().filter(|u| !u.is_empty()).unwrap_or_else(|| "inch".into())),
                Cell::Num(l.qty_or_one()),
                opt_num(l.duration_minutes),
                from_rates('E', Cell::text(&l.unit)),
                from_rates('F', Cell::Num(l.rate)),
                from_rates('G', Cell::Num(l.wastage_pct)),
                opt_num(l.mrp.filter(|m| *m > Decimal::ZERO)),
                Cell::formula(line_total(n), Cached::Num(total)),
            ]);
        }
        sums.insert(p.code.to_string(), sum);
    }
    (Sheet { name: LINES_SHEET.into(), columns: cols, rows: out, blank_rows: 1000 }, sums)
}

// ----------------------------------------------------- Product Master tab

fn lines_ref(header: &str) -> String {
    let c = letter(line_col(header));
    format!("'{LINES_SHEET}'!${c}:${c}")
}

/// The product's Cost Lines of one Type, added up; blank when it has none.
fn stage_formula(label: &str, r: usize) -> String {
    let (codes, types, totals) = (lines_ref("Product Code"), lines_ref("Type"), lines_ref("Line Total"));
    format!(r#"IF(COUNTIFS({codes},$A{r},{types},"{label}")=0,"",SUMIFS({totals},{codes},$A{r},{types},"{label}"))"#)
}

/// Each stage rounded on its own, as `calculate_cost_sheet_totals` does. Blank without lines.
fn cost_formula(r: usize) -> String {
    let stages: Vec<String> = (0..SHEET_STAGES.len())
        .map(|i| {
            let (a, o) = (letter(STAGE_COL + 2 * i), letter(STAGE_COL + 2 * i + 1));
            format!("ROUND(N({a}{r})*(1+N({o}{r})),2)")
        })
        .collect();
    format!(r#"IF(OR($A{r}="",COUNTIF({},$A{r})=0),"",{})"#, lines_ref("Product Code"), stages.join("+"))
}

fn sp_formula(r: usize) -> String {
    format!(r#"IF(E{r}="","",IF(N(F{r})>=1,CEILING(E{r},10),CEILING(ROUND(E{r}/(1-N(F{r})),2),10)))"#)
}

/// What `sp_formula` works out to, for a markup given as a fraction.
fn formula_sp(cp: f64, m: f64) -> f64 {
    js::ceil10(if m >= 1.0 { cp } else { js::round2(cp / (1.0 - m)) })
}

fn master_columns() -> Vec<Column> {
    let mut cols = vec![
        Column::new("Product Code", 16),
        Column::new("Category", 20),
        Column::new("Product Name", 40),
        Column::new("Source", 18),
        Column::new("Cost Price", 12).money().calc(cost_formula),
        Column::new("Markup %", 11).percent(),
        Column::new("Selling Price", 13).money().calc(sp_formula),
        Column::new("Active", 9),
    ];
    for (_, label) in SHEET_STAGES {
        cols.push(Column::new(label, 12).money().calc(move |r| stage_formula(label, r)));
        cols.push(Column::new(format!("{label} OH %"), 11).percent());
    }
    cols
}

/// A product with its cost sheet's overheads and lines, for the download.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SheetProduct {
    pub code: String,
    pub name: String,
    pub category_name: Option<String>,
    pub source: Option<String>,
    pub cost_price: Decimal,
    pub default_sp: Decimal,
    pub markup_pct: Option<Decimal>,
    pub is_active: bool,
    pub stage_overheads: StageOverheads,
    pub lines: Vec<ProductCostLine>,
}

/// `r` is the sheet row the product lands on (2 for the first); `sums` its line totals per stage.
pub fn product_row(p: &SheetProduct, r: usize, sums: Option<&BTreeMap<String, f64>>) -> Vec<Cell> {
    let (cp, sp) = (f(p.cost_price), f(p.default_sp));
    // The stored markup, else the one implied by today's price (older products have none).
    let stored = fo(p.markup_pct) / 100.0;
    let m = if stored > 0.0 {
        stored
    } else if sp > 0.0 {
        (sp - cp) / sp
    } else {
        0.0
    };
    let stages: Vec<(&str, Option<f64>, f64)> = SHEET_STAGES
        .iter()
        .map(|(code, label)| {
            let amount = sums.and_then(|s| s.get(*code)).map(|a| js::round2(*a));
            (*label, amount, fo(p.stage_overheads.get(*code).copied()))
        })
        .collect();
    let lines_cost =
        js::round2(stages.iter().fold(0.0, |s, (_, a, oh)| s + js::round2(a.unwrap_or(0.0) * (1.0 + oh / 100.0))));
    let live_cost = sums.is_some_and(|s| !s.is_empty()) && lines_cost == js::round2(cp);
    let live_sp = cp > 0.0 && sp > 0.0 && formula_sp(cp, m) == sp;

    let mut row = vec![
        Cell::text(&p.code),
        Cell::text(p.category_name.clone().unwrap_or_default()),
        Cell::text(&p.name),
        Cell::text(p.source.clone().unwrap_or_default()),
        if live_cost { Cell::formula(cost_formula(r), Cached::Num(dec(cp))) } else { dec(cp).into() },
        // Unrounded, so Excel does the same arithmetic as formula_sp.
        if m == 0.0 { Cell::Empty } else { dec(m).into() },
        if live_sp { Cell::formula(sp_formula(r), Cached::Num(dec(sp))) } else { dec(sp).into() },
        Cell::text(if p.is_active { "Yes" } else { "No" }),
    ];
    for (label, amount, overhead) in stages {
        let cached = amount.map_or_else(|| Cached::Text(String::new()), |a| Cached::Num(dec(a)));
        row.push(Cell::formula(stage_formula(label, r), cached));
        row.push(if overhead == 0.0 { Cell::Empty } else { dec(overhead / 100.0).into() });
    }
    row
}

/// The three tabs: Product Master, Cost Lines, Rates.
pub fn product_sheets(products: &[SheetProduct], rates: &CostRows) -> Vec<Sheet> {
    let lines: Vec<ProductLines> =
        products.iter().map(|p| ProductLines { code: &p.code, name: &p.name, lines: &p.lines }).collect();
    let (lines_tab, sums) = lines_sheet(&lines, rates);
    let cols = master_columns();
    // A blank row between products; a product's colours (LC/0001/WL, LC/0001/BL) stay together.
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut family = String::new();
    for p in products {
        let parsed = parse_product_code(&p.code);
        let next = if parsed.is_valid { format!("{}/{}", parsed.category_code, parsed.serial) } else { p.code.clone() };
        if !rows.is_empty() && next != family {
            rows.push(formula_row(&cols, rows.len() + 2));
        }
        family = next;
        rows.push(product_row(p, rows.len() + 2, sums.get(&p.code)));
    }
    vec![Sheet { name: SHEET_NAME.into(), columns: cols, rows, blank_rows: 300 }, lines_tab, rates_sheet(rates)]
}

// ----------------------------------------------------------- reading back

/// "10.00%" is 10; a bare 0.1 or 10 is read the way Markup % is.
fn pct(v: &str) -> Decimal {
    dec(js::round2(if v.ends_with('%') { js::num_str(v) } else { f(normalize_pct_str(v)) * 100.0 }))
}

/// The Product Master row's overhead % per stage; blank and 0 are left out.
pub fn read_overheads(row: &[String]) -> StageOverheads {
    let row = row.to_vec();
    SHEET_STAGES
        .iter()
        .enumerate()
        .filter_map(|(i, (code, _))| {
            let v = cell(Some(&row), STAGE_COL + 2 * i + 1);
            let p = pct(&v);
            (!v.is_empty() && !p.is_zero()).then(|| ((*code).to_string(), p))
        })
        .collect()
}

/// The Cost Lines tab as each product's lines. An item on the Rates page
/// takes its rate, unit and wastage from there, as the calculator does; one
/// that is not keeps what was typed, with a warning. An unknown Type is an error.
pub fn read_cost_lines(
    grid: Option<&Grid>,
    stages: &[StageNode],
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) -> BTreeMap<String, Vec<ProductCostLine>> {
    let mut stage_by: HashMap<String, &StageNode> = HashMap::new();
    for s in stages {
        for k in [norm(&s.row.code), norm(&s.row.name), norm(stage_label(&s.row.code, &s.row.code))] {
            stage_by.insert(k, s);
        }
    }
    let mut lines: BTreeMap<String, Vec<ProductCostLine>> = BTreeMap::new();
    for (row, n) in numbered(grid) {
        let get = |h: &str| cell(Some(row), line_col(h));
        let code = get("Product Code");
        if code.is_empty() || (get("Type").is_empty() && get("Category").is_empty() && get("Variety").is_empty()) {
            continue;
        }
        let Some(stage) = stage_by.get(&norm(&get("Type"))) else {
            let labels: Vec<&str> = SHEET_STAGES.iter().map(|(_, l)| *l).collect();
            errors.push(format!("Cost Lines row {n}: Type \"{}\" isn't one of {}.", get("Type"), labels.join(", ")));
            continue;
        };
        let cat = stage.categories.iter().find(|c| norm(&c.row.name) == norm(&get("Category")));
        let sub = cat.and_then(|c| c.subcategories.iter().find(|s| norm(&s.row.name) == norm(&get("Subcategory"))));
        let v = sub.and_then(|s| s.varieties.iter().find(|x| norm(&x.name) == norm(&get("Variety"))));
        let sub_name = sub.map_or_else(|| get("Subcategory"), |s| s.row.name.clone());
        let var_name = v.map_or_else(|| get("Variety"), |v| v.name.clone());
        let bought = stage.row.code == "bought_out";
        let size = norm(&get("Size In"));
        let num_or_none = |h: &str| Some(get(h)).filter(|s| !s.is_empty()).map(|s| parse_num(&s));
        let item_name = [sub_name.as_str(), var_name.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        let typed_unit = get("Unit");
        let mrp = parse_num(&get("MRP"));
        let mut line = ProductCostLine {
            stage_code: stage.row.code.clone(),
            category_name: cat.map_or_else(|| get("Category"), |c| c.row.name.clone()),
            subcategory_name: Some(sub_name.clone()).filter(|s| !s.is_empty()),
            variety_name: Some(var_name.clone()).filter(|s| !s.is_empty()),
            item_name: if item_name.is_empty() { get("Category") } else { item_name },
            cost_variety_id: v.map(|v| v.id.clone()),
            length: num_or_none("Length"),
            breadth: num_or_none("Breadth"),
            dimension_unit: Some(if size == "mm" || size == "cm" { size } else { "inch".into() }),
            unit: v.map(|v| v.unit.clone()).unwrap_or_else(|| {
                if !typed_unit.is_empty() {
                    typed_unit
                } else if stage.row.code == "machine" {
                    "min".into()
                } else {
                    "piece".into()
                }
            }),
            rate: v.map_or_else(|| parse_num(&get("Rate")), |v| v.default_rate),
            duration_minutes: num_or_none("Minutes"),
            qty: Some(if get("Qty").is_empty() { Decimal::ONE } else { parse_num(&get("Qty")) }),
            wastage_pct: if bought {
                Decimal::ZERO
            } else {
                v.map_or_else(|| parse_num(&get("Wastage %")), |v| v.default_wastage_pct)
            },
            mrp: (mrp > Decimal::ZERO).then_some(mrp),
            ..Default::default()
        };
        if v.is_none() {
            let path: Vec<String> = ["Type", "Category", "Subcategory", "Variety"]
                .iter()
                .map(|h| get(h))
                .filter(|s| !s.is_empty())
                .collect();
            warnings.push(format!(
                "{code}: {} isn't on the Rates page, so its typed rate ₹{} is used.",
                path.join(" › "),
                js_num(line.rate)
            ));
        }
        let calc = calculate_line_cost(&line);
        let list = lines.entry(code).or_default();
        line.calculated_area = Some(calc.calculated_area);
        line.line_total = calc.line_total;
        line.sort_order = Some(i32::try_from(list.len() + 1).unwrap_or(i32::MAX));
        list.push(line);
    }
    lines
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExistingCosting {
    pub sheet_id: String,
    pub overheads: StageOverheads,
    pub lines: Vec<ProductCostLine>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostingPlan {
    pub code: String,
    pub sheet_id: Option<String>,
    pub overheads: StageOverheads,
    pub totals: CostSheetTotals,
    pub lines: Vec<ProductCostLine>,
}

/// What a line is, ignoring ids and order numbers, so a round trip compares equal.
#[derive(PartialEq)]
struct Sig {
    text: [String; 7],
    nums: [Decimal; 7],
}

fn sig(l: &ProductCostLine) -> Sig {
    Sig {
        text: [
            l.stage_code.clone(),
            l.category_name.clone(),
            l.subcategory_name.clone().unwrap_or_default(),
            l.variety_name.clone().unwrap_or_default(),
            l.cost_variety_id.clone().unwrap_or_default(),
            l.dimension_unit.clone().filter(|u| !u.is_empty()).unwrap_or_else(|| "inch".into()),
            l.unit.clone(),
        ],
        nums: [
            l.length.unwrap_or_default(),
            l.breadth.unwrap_or_default(),
            l.rate,
            l.duration_minutes.unwrap_or_default(),
            l.qty_or_one(),
            l.wastage_pct,
            l.mrp.unwrap_or_default(),
        ],
    }
}

fn same_overheads(a: &StageOverheads, b: &StageOverheads) -> bool {
    a.keys().chain(b.keys()).all(|k| a.get(k).copied().unwrap_or_default() == b.get(k).copied().unwrap_or_default())
}

/// A product row of the sheet, for [`plan_sheet_costing`].
pub struct SheetCost<'a> {
    pub code: &'a str,
    pub cost_price: Decimal,
    pub markup_pct: Decimal,
}

/// A product's costing from the sheet: its lines replace the saved ones, with
/// the row's overheads. None when that is what is saved already.
pub fn plan_sheet_costing(
    p: &SheetCost,
    lines: &[ProductCostLine],
    overheads: &StageOverheads,
    existing: Option<&ExistingCosting>,
    warnings: &mut Vec<String>,
) -> Option<CostingPlan> {
    if let Some(e) = existing {
        let mut saved: Vec<&ProductCostLine> = e.lines.iter().collect();
        saved.sort_by_key(|l| l.sort_order.unwrap_or(0));
        let unchanged = same_overheads(overheads, &e.overheads)
            && saved.len() == lines.len()
            && saved.iter().zip(lines).all(|(a, b)| sig(a) == sig(b));
        if unchanged {
            return None;
        }
    }
    let totals = calculate_cost_sheet_totals(lines, p.markup_pct, overheads);
    // Excel works the same sums out; a gap means a typed-over Cost Price or a rate the Rates page changed.
    if (f(p.cost_price) - f(totals.total_cost)).abs() >= 0.5 {
        let total = js_num(totals.total_cost);
        warnings.push(format!(
            "{}: its lines cost ₹{total}, not the ₹{} in Cost Price; ₹{total} is saved.",
            p.code,
            js_num(p.cost_price)
        ));
    }
    Some(CostingPlan {
        code: p.code.to_string(),
        sheet_id: existing.map(|e| e.sheet_id.clone()),
        overheads: overheads.clone(),
        totals,
        lines: lines.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost_tree::{CalculationType, CostCategory, CostStage, CostSubcategory};
    use crate::import::{ImportInput, build_import};
    use crate::sheets::testing::grid;
    use rust_decimal_macros::dec;

    fn rates() -> CostRows {
        CostRows {
            stages: vec![
                CostStage {
                    id: "st1".into(),
                    code: "material".into(),
                    name: "Material".into(),
                    sort_order: 1,
                    is_active: true,
                    ..Default::default()
                },
                CostStage {
                    id: "st2".into(),
                    code: "machine".into(),
                    name: "Machine Cost".into(),
                    calculation_type: CalculationType::TimeBased,
                    sort_order: 2,
                    is_active: true,
                },
            ],
            categories: vec![CostCategory {
                id: "c1".into(),
                stage_id: "st1".into(),
                name: "Woodbased".into(),
                sort_order: 0,
                is_active: true,
            }],
            subcategories: vec![CostSubcategory {
                id: "s1".into(),
                category_id: "c1".into(),
                name: "Birch".into(),
                sort_order: 0,
                is_active: true,
            }],
            varieties: vec![CostVariety {
                id: "v1".into(),
                subcategory_id: "s1".into(),
                name: "3mm".into(),
                default_rate: dec!(90),
                unit: "sq ft".into(),
                default_wastage_pct: dec!(10),
                notes: None,
                sort_order: 0,
                is_active: true,
            }],
        }
    }

    fn product(code: &str, cp: Decimal, sp: Decimal, markup: Decimal) -> SheetProduct {
        SheetProduct {
            code: code.into(),
            name: format!("Item {code} & <more>"),
            category_name: Some("Utility".into()),
            cost_price: cp,
            default_sp: sp,
            markup_pct: Some(markup),
            is_active: true,
            ..Default::default()
        }
    }

    fn formula_of(c: Option<&Cell>) -> Option<String> {
        match c {
            Some(Cell::Formula { f, .. }) => Some(f.clone()),
            _ => None,
        }
    }

    #[test]
    fn sp_formula_only_where_it_gives_todays_price() {
        let sheets = product_sheets(
            &[
                product("TS/0001/WL", dec!(16.13), dec!(100), dec!(83)),
                product("TS/0001/BL", dec!(250), dec!(400), dec!(0)),
                product("TS/0002/WL", dec!(250), dec!(395), dec!(0)),
                product("TS/0003/WL", dec!(0), dec!(50), dec!(0)),
            ],
            &CostRows::default(),
        );
        let master = &sheets[0];
        assert_eq!(master.name, SHEET_NAME);
        let g = grid(master);
        assert_eq!(g[0][5], "Markup %");
        let rows = build_import(&ImportInput { products: Some(g), ..Default::default() }).products;
        assert_eq!(rows.len(), 4, "gap rows are ignored");
        assert_eq!(rows[0].name, "Item TS/0001/WL & <more>");
        assert_eq!(rows.iter().map(|r| r.default_sp).collect::<Vec<_>>(), [dec!(100), dec!(400), dec!(395), dec!(50)]);
        assert_eq!(rows[0].target_margin, dec!(0.83));

        let r = &master.rows;
        assert!(formula_of(r[0].get(6)).is_some_and(|f| f.contains("CEILING")), "G2 is the SP formula");
        assert!(formula_of(r[1].get(6)).is_some(), "the other colour follows straight on");
        assert!(r[2].first() == Some(&Cell::Empty) && formula_of(r[2].get(4)).is_some(), "a gap carrying formulas");
        assert_eq!(r[3].get(6), Some(&Cell::Num(dec!(395))));
        assert_eq!(r[5].get(6), Some(&Cell::Num(dec!(50))));
        assert_eq!(r.len(), 6, "4 products and 2 gaps; the writer adds the blank rows after");
        // Row 8 is the first trailing blank row: it carries the SP and lines-sum cost formulas.
        let blank = formula_row(&master.columns, 8);
        assert!(formula_of(blank.get(6)).is_some());
        assert!(formula_of(blank.get(4)).is_some_and(|f| f.starts_with(r#"IF(OR($A8="",COUNTIF('Cost Lines'!"#)));
    }

    fn line(over: impl FnOnce(&mut ProductCostLine)) -> ProductCostLine {
        let mut l = ProductCostLine {
            stage_code: "material".into(),
            category_name: "Woodbased".into(),
            subcategory_name: Some("Birch".into()),
            variety_name: Some("3mm".into()),
            item_name: "Birch 3mm".into(),
            cost_variety_id: Some("v1".into()),
            length: Some(dec!(24)),
            breadth: Some(dec!(12)),
            dimension_unit: Some("inch".into()),
            unit: "sq ft".into(),
            rate: dec!(90),
            qty: Some(dec!(1)),
            wastage_pct: dec!(10),
            line_total: dec!(198),
            sort_order: Some(1),
            ..Default::default()
        };
        over(&mut l);
        l
    }

    #[test]
    fn cost_lines_and_overheads_round_trip_without_moving_the_cost() {
        let lines = vec![
            line(|_| {}),
            line(|l| {
                l.stage_code = "machine".into();
                l.category_name = "Laser".into();
                l.subcategory_name = None;
                l.variety_name = Some("Cutting".into());
                l.cost_variety_id = None;
                l.length = None;
                l.breadth = None;
                l.unit = "hour".into();
                l.rate = dec!(120);
                l.duration_minutes = Some(dec!(30));
                l.wastage_pct = dec!(0);
                l.line_total = dec!(60);
                l.sort_order = Some(2);
            }),
        ];
        // 198 × 1.1 overhead + 60
        let costed = SheetProduct {
            stage_overheads: StageOverheads::from([("material".into(), dec!(10))]),
            lines: lines.clone(),
            ..product("E", dec!(277.8), dec!(560), dec!(50))
        };
        let other = SheetProduct {
            lines: vec![line(|l| {
                l.cost_variety_id = None;
                l.line_total = dec!(12);
                l.rate = dec!(6);
            })],
            ..product("G", dec!(12), dec!(30), dec!(0))
        };
        let sheets = product_sheets(&[costed, product("F", dec!(99), dec!(200), dec!(0)), other], &rates());
        assert_eq!(sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), [SHEET_NAME, LINES_SHEET, RATES_SHEET]);

        let master = grid(&sheets[0]);
        assert_eq!((master[0][8].as_str(), master[0][9].as_str()), ("Material", "Material OH %"));
        assert_eq!((master[1][8].as_str(), master[1][14].as_str()), ("198.00", "60.00"));
        assert!(formula_of(sheets[0].rows[0].get(4)).is_some_and(|f| f.starts_with("IF(OR(")), "E2 is the formula");
        assert_eq!(sheets[0].rows[1].first(), Some(&Cell::Empty), "a gap row between products");
        assert_eq!(formula_of(sheets[0].rows[2].get(4)), None, "no lines: a typed cost");

        let ws = &sheets[1].rows;
        assert!(matches!(ws[0].get(1), Some(Cell::Formula { v: Some(Cached::Text(n)), .. }) if n == "Item E & <more>"));
        assert!(formula_of(ws[0].get(1)).is_some_and(|f| f.contains("MATCH(A2,'Product Master'!$A:$A")));
        assert!(formula_of(ws[0].get(12)).is_some_and(|f| f.contains("MATCH(")), "a master item's rate is looked up");
        assert_eq!(formula_of(ws[1].get(12)), None, "an item not on the Rates page keeps its typed rate");
        assert_eq!(ws[2].first(), Some(&Cell::Empty), "a gap before the next product's lines");
        assert_eq!(ws[3].first(), Some(&Cell::text("G")));
        assert_eq!(
            grid(&sheets[2])[1].join(","),
            "Material,Woodbased,Birch,3mm,sq ft,90.00,10,Material|Woodbased|Birch|3mm"
        );

        let result = build_import(&ImportInput { products: Some(master.clone()), ..Default::default() });
        let (mut warnings, mut errors) = (Vec::new(), Vec::new());
        let read = read_cost_lines(Some(&grid(&sheets[1])), &build_cost_tree(&rates()), &mut warnings, &mut errors);
        let e = read.get("E").cloned().unwrap_or_default();
        assert_eq!(e.iter().map(|l| l.line_total).collect::<Vec<_>>(), [dec!(198), dec!(60)]);
        assert_eq!(result.products[0].cost_price, dec!(277.8));
        let saved = ExistingCosting {
            sheet_id: "s1".into(),
            overheads: StageOverheads::from([("material".into(), dec!(10))]),
            lines,
        };
        let p = SheetCost { code: "E", cost_price: result.products[0].cost_price, markup_pct: dec!(50) };
        assert_eq!(plan_sheet_costing(&p, &e, &read_overheads(&master[1]), Some(&saved), &mut warnings), None);
    }

    fn line_row(cells: &[(&str, &str)]) -> Vec<String> {
        LINE_HEADERS
            .iter()
            .map(|h| cells.iter().find(|(k, _)| k == h).map(|(_, v)| (*v).to_string()).unwrap_or_default())
            .collect()
    }

    fn lines_grid(rows: Vec<Vec<String>>) -> Grid {
        let mut g = vec![LINE_HEADERS.iter().map(|h| (*h).to_string()).collect()];
        g.extend(rows);
        g
    }

    #[test]
    fn read_overheads_skips_blank_and_zero() {
        let mut r = vec![String::new(); 20];
        r[9] = "12.50%".into();
        r[13] = "0.05".into();
        r[15] = "0".into();
        assert_eq!(
            read_overheads(&r),
            StageOverheads::from([("material".into(), dec!(12.5)), ("finishing".into(), dec!(5))])
        );
    }

    #[test]
    fn master_items_take_the_master_rate_and_others_keep_theirs() {
        let (mut warnings, mut errors) = (Vec::new(), Vec::new());
        let g = lines_grid(vec![
            line_row(&[
                ("Product Code", "A"),
                ("Type", "material"),
                ("Category", "woodbased"),
                ("Subcategory", "BIRCH"),
                ("Variety", "3mm"),
                ("Length", "12"),
                ("Breadth", "12"),
                ("Rate", "1"),
                ("Unit", "piece"),
            ]),
            line_row(&[
                ("Product Code", "A"),
                ("Type", "Machine Cost"),
                ("Category", "Laser"),
                ("Variety", "Cutting"),
                ("Minutes", "30"),
                ("Unit", "hour"),
                ("Rate", "120"),
            ]),
            line_row(&[
                ("Product Code", "B"),
                ("Type", "Material"),
                ("Category", "Woodbased"),
                ("Subcategory", "Birch"),
                ("Variety", "3mm"),
                ("Qty", "0"),
            ]),
            line_row(&[("Product Code", "B")]),
        ]);
        let lines = read_cost_lines(Some(&g), &build_cost_tree(&rates()), &mut warnings, &mut errors);
        assert!(errors.is_empty());
        let a = &lines["A"];
        let (birch, laser) = (&a[0], &a[1]);
        assert_eq!(birch.cost_variety_id.as_deref(), Some("v1"));
        assert_eq!(
            (
                birch.category_name.as_str(),
                birch.subcategory_name.as_deref(),
                birch.unit.as_str(),
                birch.rate,
                birch.wastage_pct
            ),
            ("Woodbased", Some("Birch"), "sq ft", dec!(90), dec!(10))
        );
        assert_eq!(birch.line_total, dec!(99), "1 sq ft × 90 + 10%");
        assert_eq!(
            (
                laser.stage_code.as_str(),
                laser.cost_variety_id.as_deref(),
                laser.unit.as_str(),
                laser.line_total,
                laser.sort_order
            ),
            ("machine", None, "hour", dec!(60), Some(2))
        );
        assert_eq!(lines["B"].len(), 1);
        assert_eq!(lines["B"][0].line_total, dec!(0), "qty 0 costs nothing");
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].starts_with(
                "A: Machine Cost › Laser › Cutting isn't on the Rates page, so its typed rate ₹120 is used"
            )
        );

        let bad = lines_grid(vec![line_row(&[("Product Code", "C"), ("Type", "Matrial"), ("Category", "Woodbased")])]);
        read_cost_lines(Some(&bad), &build_cost_tree(&rates()), &mut Vec::new(), &mut errors);
        assert!(errors[0].contains("row 2: Type \"Matrial\" isn't one of Material, Hardware"));
    }

    #[test]
    fn unchanged_costing_is_none_and_changes_replace_it() {
        let g = lines_grid(vec![line_row(&[
            ("Product Code", "A"),
            ("Type", "Material"),
            ("Category", "Woodbased"),
            ("Subcategory", "Birch"),
            ("Variety", "3mm"),
            ("Length", "24"),
            ("Breadth", "12"),
        ])]);
        let lines =
            read_cost_lines(Some(&g), &build_cost_tree(&rates()), &mut Vec::new(), &mut Vec::new())["A"].clone();
        let p = SheetCost { code: "A", cost_price: dec!(217.8), markup_pct: dec!(50) };
        let ten = StageOverheads::from([("material".into(), dec!(10))]);
        let saved = ExistingCosting {
            sheet_id: "s1".into(),
            overheads: ten.clone(),
            lines: lines
                .iter()
                .map(|l| ProductCostLine { id: Some("x".into()), sheet_id: Some("s1".into()), ..l.clone() })
                .collect(),
        };
        assert_eq!(
            plan_sheet_costing(&p, &lines, &ten, Some(&saved), &mut Vec::new()),
            None,
            "a round trip writes nothing"
        );

        let mut warnings = Vec::new();
        let plan = plan_sheet_costing(&p, &lines, &StageOverheads::new(), Some(&saved), &mut warnings);
        let plan = plan.unwrap();
        assert_eq!(plan.sheet_id.as_deref(), Some("s1"));
        assert_eq!(plan.totals.total_cost, dec!(198));
        assert!(warnings[0].contains("lines cost ₹198, not the ₹217.8 in Cost Price"));

        let fresh = SheetCost { cost_price: dec!(198), ..p };
        let plan = plan_sheet_costing(&fresh, &lines, &StageOverheads::new(), None, &mut warnings);
        assert_eq!(plan.as_ref().map(|p| p.totals.total_cost), Some(dec!(198)));
        assert_eq!(plan.and_then(|p| p.sheet_id), None, "no sheet yet: one is created");
        assert_eq!(warnings.len(), 1, "Cost Price agrees: no warning");
    }
}
