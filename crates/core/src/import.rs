//! The one-time spreadsheet import: six exported sheets → rows for the
//! database (port of `import.ts`, `import-cells.ts`, `import-quotes.ts`).
//!
//! Columns are read by POSITION, matching the indices the Apps Script used:
//! header text is decorative and people rename headers. Row 1 is the header
//! in every sheet. Pure, so the page can show a dry run before writing.

use crate::js::{self, dec, f};
use crate::numbers::{normalize_pct_str, parse_num};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::{Date, Month};

pub type Grid = Vec<Vec<String>>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportInput {
    pub settings: Option<Grid>,
    pub products: Option<Grid>,
    pub hamper_summary: Option<Grid>,
    pub hamper_details: Option<Grid>,
    pub quotation_register: Option<Grid>,
    pub quote_details: Option<Grid>,
}

/// A trimmed cell; missing cells read as "".
pub fn cell(row: Option<&Vec<String>>, i: usize) -> String {
    row.and_then(|r| r.get(i)).map(|c| c.trim().to_string()).unwrap_or_default()
}

fn at(row: &[String], i: usize) -> String {
    row.get(i).map(|c| c.trim().to_string()).unwrap_or_default()
}

fn opt(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}

pub fn is_yes(v: &str) -> bool {
    matches!(v.trim().to_lowercase().as_str(), "yes" | "y" | "true" | "1" | "active")
}

fn filled(row: &[String]) -> bool {
    row.iter().any(|c| !c.trim().is_empty())
}

/// Rows after the header, skipping blank ones.
pub fn body(grid: Option<&Grid>) -> Vec<&Vec<String>> {
    grid.map(|g| g.iter().skip(1).filter(|r| filled(r)).collect()).unwrap_or_default()
}

/// Like [`body`], with each row's 1-based number in the sheet. The grid must keep blank rows.
pub fn numbered(grid: Option<&Grid>) -> Vec<(&Vec<String>, usize)> {
    grid.map(|g| g.iter().enumerate().skip(1).filter(|(_, r)| filled(r)).map(|(i, r)| (r, i + 1)).collect())
        .unwrap_or_default()
}

fn month_named(s: &str) -> Option<Month> {
    let m = s.get(..3)?.to_lowercase();
    Some(match m.as_str() {
        "jan" => Month::January,
        "feb" => Month::February,
        "mar" => Month::March,
        "apr" => Month::April,
        "may" => Month::May,
        "jun" => Month::June,
        "jul" => Month::July,
        "aug" => Month::August,
        "sep" => Month::September,
        "oct" => Month::October,
        "nov" => Month::November,
        "dec" => Month::December,
        _ => return None,
    })
}

fn ymd(y: &str, m: Month, d: &str) -> Option<String> {
    let date = Date::from_calendar_date(y.parse().ok()?, m, d.parse().ok()?).ok()?;
    Some(format!("{:04}-{:02}-{:02}", date.year(), u8::from(date.month()), date.day()))
}

fn month_num(s: &str) -> Option<Month> {
    Month::try_from(s.parse::<u8>().ok()?).ok()
}

/// Sheets export dates in whatever the locale was. ISO first, then D/M/Y (the
/// Indian convention this business uses), then "5 Jan 2025" / "Jan 5, 2025".
/// Anything else, or an impossible date, is reported rather than guessed at.
pub fn parse_date(value: &str) -> Option<String> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    let b = raw.as_bytes();
    let is_digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    if (0..4).all(is_digit) && b.get(4) == Some(&b'-') && is_digit(5) && is_digit(6) && b.get(7) == Some(&b'-') {
        let (y, m, d) = (raw.get(0..4)?, raw.get(5..7)?, raw.get(8..10)?);
        return ymd(y, month_num(m)?, d);
    }

    let words: Vec<&str> = raw.split(['/', '-', ' ', ',', 'T']).filter(|w| !w.is_empty()).collect();
    match words.as_slice() {
        [d, m, y, ..] if d.len() <= 2 && y.len() == 4 && d.chars().all(|c| c.is_ascii_digit()) => {
            let month = if m.chars().all(|c| c.is_ascii_digit()) { month_num(m) } else { month_named(m) };
            ymd(y, month?, d)
        }
        [m, d, y, ..] if !m.chars().all(|c| c.is_ascii_digit()) && y.len() == 4 => ymd(y, month_named(m)?, d),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CategoryRow {
    pub name: String,
    pub counts_as_item: bool,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductRow {
    pub code: String,
    pub category_name: Option<String>,
    pub name: String,
    pub source: Option<String>,
    pub cost_price: Decimal,
    pub target_margin: Decimal,
    pub default_sp: Decimal,
    pub is_active: bool,
    /// Whole percent; only the Product Master Excel sheet sets it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markup_pct: Option<Decimal>,
    /// Set only for a product the Excel sheet creates, from its code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HamperRow {
    pub code: String,
    pub name: String,
    pub collection: Option<String>,
    pub status: String,
    pub discount_pct: Decimal,
    pub target_sp: Option<Decimal>,
    pub final_catalogue_sp: Option<Decimal>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HamperItemRow {
    pub hamper_code: String,
    pub line_no: i32,
    pub product_code: Option<String>,
    pub product_name: String,
    pub category_name: Option<String>,
    pub source: Option<String>,
    pub qty: Decimal,
    pub unit_cp: Decimal,
    pub unit_sp: Decimal,
    pub target_margin: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuoteRow {
    pub doc_no: String,
    pub doc_type: crate::records::DocType,
    pub doc_date: Option<String>,
    pub client_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub occasion: Option<String>,
    pub quote_structure: String,
    pub status: String,
    pub validity: Option<String>,
    pub follow_up_date: Option<String>,
    pub notes: Option<String>,
    pub linked_doc_no: Option<String>,
    pub gst_rate: Decimal,
    pub order_discount: Decimal,
    pub adj1: Decimal,
    pub adj2: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuoteItemRow {
    pub doc_no: String,
    pub line_no: i32,
    pub option_label: Option<String>,
    pub hamper_code: Option<String>,
    pub hamper_name: Option<String>,
    pub qty: Decimal,
    pub catalogue_price: Decimal,
    pub discount_pct: Decimal,
    pub detail_mode: Option<String>,
    pub packaging_treatment: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportCounts {
    pub categories: usize,
    pub products: usize,
    pub hampers: usize,
    pub hamper_items: usize,
    pub quotes: usize,
    pub quote_items: usize,
}

/// Rows ready to write, plus warnings for the dry-run report.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportResult {
    pub categories: Vec<CategoryRow>,
    pub default_detail_mode: Option<String>,
    pub products: Vec<ProductRow>,
    pub hampers: Vec<HamperRow>,
    pub hamper_items: Vec<HamperItemRow>,
    pub quotes: Vec<QuoteRow>,
    pub quote_items: Vec<QuoteItemRow>,
    pub warnings: Vec<String>,
    pub counts: ImportCounts,
}

/// Values in first-seen order, each once.
fn unique<'a>(values: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    values.filter(|v| seen.insert(*v)).collect()
}

fn find_duplicates<'a>(values: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    unique(values.filter(|v| !seen.insert(*v)))
}

/// A running 1-based line number per key, counted before rows are filtered
/// (as the sheet numbered them).
fn line_counter() -> impl FnMut(&str) -> i32 {
    let mut counts: HashMap<String, i32> = HashMap::new();
    move |key| {
        let n = counts.entry(key.to_string()).or_insert(0);
        *n += 1;
        *n
    }
}

fn opt_num(s: String) -> Option<Decimal> {
    (!s.is_empty()).then(|| parse_num(&s))
}

pub fn build_import(input: &ImportInput) -> ImportResult {
    let mut warnings = Vec::new();

    // Settings: A = category, B = counts as item, H2 = default detail mode.
    let categories: Vec<CategoryRow> = body(input.settings.as_ref())
        .iter()
        .enumerate()
        .map(|(i, row)| CategoryRow {
            name: at(row, 0),
            counts_as_item: at(row, 1).is_empty() || is_yes(&at(row, 1)),
            sort_order: i32::try_from(i).unwrap_or(i32::MAX),
        })
        .filter(|c| !c.name.is_empty())
        .collect();
    let default_detail_mode = opt(cell(input.settings.as_ref().and_then(|g| g.get(1)), 7));

    // Product Master: A code, B category, C name, D source, E cost, F margin, G SP, H active.
    let products: Vec<ProductRow> = body(input.products.as_ref())
        .iter()
        .map(|row| ProductRow {
            code: at(row, 0),
            category_name: opt(at(row, 1)),
            name: at(row, 2),
            source: opt(at(row, 3)),
            cost_price: parse_num(&at(row, 4)),
            target_margin: normalize_pct_str(&at(row, 5)),
            default_sp: parse_num(&at(row, 6)),
            is_active: at(row, 7).is_empty() || is_yes(&at(row, 7)),
            markup_pct: None,
            colors: None,
        })
        .filter(|p| !p.code.is_empty() && !p.name.is_empty())
        .collect();

    let dupes = find_duplicates(products.iter().map(|p| p.code.as_str()));
    if !dupes.is_empty() {
        warnings
            .push(format!("Product Master has repeated codes, only the last of each is kept: {}", dupes.join(", ")));
    }

    // Hamper Summary: only the input columns. The rest is recalculated by the view.
    let hampers: Vec<HamperRow> = body(input.hamper_summary.as_ref())
        .iter()
        .map(|row| HamperRow {
            code: at(row, 0),
            name: at(row, 1),
            collection: opt(at(row, 2)),
            status: opt(at(row, 3)).unwrap_or_else(|| "Draft".into()),
            discount_pct: normalize_pct_str(&at(row, 7)),
            target_sp: opt_num(at(row, 9)),
            final_catalogue_sp: opt_num(at(row, 11)),
            notes: opt(at(row, 14)),
        })
        .filter(|h| !h.code.is_empty() && !h.name.is_empty())
        .collect();

    // Hamper Details: the price snapshots, kept exactly as saved.
    let mut next_line = line_counter();
    let all_items: Vec<HamperItemRow> = body(input.hamper_details.as_ref())
        .iter()
        .map(|row| {
            let hamper_code = at(row, 0);
            HamperItemRow {
                line_no: next_line(&hamper_code),
                hamper_code,
                product_code: opt(at(row, 4)),
                product_name: at(row, 6),
                category_name: opt(at(row, 5)),
                source: opt(at(row, 8)),
                qty: parse_num(&at(row, 7)),
                unit_cp: parse_num(&at(row, 9)),
                target_margin: normalize_pct_str(&at(row, 11)),
                unit_sp: parse_num(&at(row, 12)),
            }
        })
        .filter(|i| !i.hamper_code.is_empty() && !i.product_name.is_empty())
        .collect();

    let hamper_codes: HashSet<&str> = hampers.iter().map(|h| h.code.as_str()).collect();
    let orphans = unique(all_items.iter().map(|i| i.hamper_code.as_str()).filter(|c| !hamper_codes.contains(c)));
    if !orphans.is_empty() {
        warnings.push(format!(
            "Hamper Details refers to hampers missing from Hamper Summary, these lines are skipped: {}",
            orphans.join(", ")
        ));
    }

    let product_codes: HashSet<&str> = products.iter().map(|p| p.code.as_str()).collect();
    let missing =
        unique(all_items.iter().filter_map(|i| i.product_code.as_deref()).filter(|c| !product_codes.contains(c)));
    if !missing.is_empty() {
        let shown: Vec<&str> = missing.iter().take(8).copied().collect();
        warnings.push(format!(
            "{} product code(s) on saved hampers are not in Product Master. The saved prices are kept, but Refresh Prices will flag them: {}{}",
            missing.len(),
            shown.join(", "),
            if missing.len() > 8 { "…" } else { "" }
        ));
    }

    let hamper_items: Vec<HamperItemRow> =
        all_items.iter().filter(|i| hamper_codes.contains(i.hamper_code.as_str())).cloned().collect();
    let (quotes, quote_items) = build_quotes(input, &mut warnings);

    let counts = ImportCounts {
        categories: categories.len(),
        products: products.len(),
        hampers: hampers.len(),
        hamper_items: hamper_items.len(),
        quotes: quotes.len(),
        quote_items: quote_items.len(),
    };
    ImportResult {
        categories,
        default_detail_mode,
        products,
        hampers,
        hamper_items,
        quotes,
        quote_items,
        warnings,
        counts,
    }
}

/// `Math.round(n * 10000) / 10000`.
fn round4(n: f64) -> f64 {
    js::round(n * 10000.0) / 10000.0
}

/// Quotation Register + Quote Details → quote rows, reconciled against their lines.
fn build_quotes(input: &ImportInput, warnings: &mut Vec<String>) -> (Vec<QuoteRow>, Vec<QuoteItemRow>) {
    let mut next_line = line_counter();
    let items: Vec<QuoteItemRow> = body(input.quote_details.as_ref())
        .iter()
        .map(|row| {
            let doc_no = at(row, 0);
            QuoteItemRow {
                line_no: next_line(&doc_no),
                doc_no,
                option_label: opt(at(row, 1)),
                hamper_code: opt(at(row, 2)),
                hamper_name: opt(at(row, 3)),
                qty: parse_num(&at(row, 4)),
                catalogue_price: parse_num(&at(row, 5)),
                discount_pct: normalize_pct_str(&at(row, 6)),
                detail_mode: opt(at(row, 9)),
                packaging_treatment: opt(at(row, 10)),
            }
        })
        .filter(|i| !i.doc_no.is_empty())
        .collect();

    let mut unparsed_dates = 0;
    let quotes: Vec<QuoteRow> = body(input.quotation_register.as_ref())
        .iter()
        .map(|row| {
            let doc_no = at(row, 0);
            let quoted_value = parse_num(&at(row, 11)); // L, the taxable value
            let gst_amount = parse_num(&at(row, 12)); // M
            let doc_date = parse_date(&at(row, 2));
            if !at(row, 2).is_empty() && doc_date.is_none() {
                unparsed_dates += 1;
            }

            // The register stored the taxable value but never the discount or
            // charges that produced it. Re-derive one adjustment so the
            // historical total still reconciles against the lines.
            let subtotal = items
                .iter()
                .filter(|i| i.doc_no == doc_no)
                .fold(0.0, |sum, i| sum + f(i.qty) * f(i.catalogue_price) * (1.0 - f(i.discount_pct)));
            let (quoted_value, gst_amount) = (f(quoted_value), f(gst_amount));
            let delta = if quoted_value == 0.0 { 0.0 } else { quoted_value - subtotal };

            QuoteRow {
                doc_type: if at(row, 1).to_lowercase().contains("proforma") {
                    crate::records::DocType::ProformaInvoice
                } else {
                    crate::records::DocType::Quotation
                },
                doc_date,
                client_name: at(row, 3),
                contact_person: opt(at(row, 4)),
                phone: opt(at(row, 5)),
                email: opt(at(row, 6)),
                occasion: opt(at(row, 7)),
                quote_structure: opt(at(row, 8)).unwrap_or_else(|| crate::pricing::COMBINED_ORDER.into()),
                status: opt(at(row, 14)).unwrap_or_else(|| "Draft".into()),
                validity: opt(at(row, 15)),
                follow_up_date: parse_date(&at(row, 16)),
                notes: opt(at(row, 17)),
                linked_doc_no: opt(at(row, 19)),
                // Recover the rate actually charged rather than assuming 18%.
                gst_rate: dec(if quoted_value > 0.0 && gst_amount > 0.0 {
                    round4(gst_amount / quoted_value)
                } else {
                    0.18
                }),
                order_discount: dec(if delta < -0.005 { delta.abs() } else { 0.0 }),
                adj1: dec(if delta > 0.005 { delta } else { 0.0 }),
                adj2: Decimal::ZERO,
                doc_no,
            }
        })
        .filter(|q| !q.doc_no.is_empty() && !q.client_name.is_empty())
        .collect();

    if unparsed_dates > 0 {
        warnings
            .push(format!("{unparsed_dates} date(s) in the Quotation Register could not be read and were left blank."));
    }
    let reconciled = quotes.iter().filter(|q| q.order_discount > Decimal::ZERO || q.adj1 > Decimal::ZERO).count();
    if reconciled > 0 {
        warnings.push(format!(
            "{reconciled} quotation(s) had a total that differs from the sum of their lines. The difference was recorded as an order discount or an extra charge so the original total is preserved."
        ));
    }
    let quote_nos: HashSet<&str> = quotes.iter().map(|q| q.doc_no.as_str()).collect();
    let orphans = unique(items.iter().map(|i| i.doc_no.as_str()).filter(|d| !quote_nos.contains(d)));
    if !orphans.is_empty() {
        warnings.push(format!(
            "Quote Details refers to documents missing from the Quotation Register, these lines are skipped: {}",
            orphans.join(", ")
        ));
    }
    let items = items.iter().filter(|i| quote_nos.contains(i.doc_no.as_str())).cloned().collect();
    (quotes, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::DocType;
    use rust_decimal_macros::dec;

    fn header(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("col{i}")).collect()
    }

    fn grid(header_len: usize, rows: &[&[&str]]) -> Option<Grid> {
        let mut g = vec![header(header_len)];
        g.extend(rows.iter().map(|r| r.iter().map(|c| (*c).to_string()).collect()));
        Some(g)
    }

    #[test]
    fn dates_read_iso_and_day_first() {
        assert_eq!(parse_date("2025-03-09").as_deref(), Some("2025-03-09"));
        assert_eq!(parse_date("2025-03-09T10:00:00Z").as_deref(), Some("2025-03-09"));
        assert_eq!(parse_date("9/3/2025").as_deref(), Some("2025-03-09"));
        assert_eq!(parse_date("09-03-2025").as_deref(), Some("2025-03-09"));
        assert_eq!(parse_date("5 Jan 2025").as_deref(), Some("2025-01-05"));
        assert_eq!(parse_date("05-Jan-2025").as_deref(), Some("2025-01-05"));
        assert_eq!(parse_date("Jan 5, 2025").as_deref(), Some("2025-01-05"));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("31/13/2025"), None, "no month 13");
        assert_eq!(parse_date("someday"), None);
    }

    #[test]
    fn settings_map_categories_and_counts_flag() {
        let r = build_import(&ImportInput {
            settings: grid(
                8,
                &[
                    &["Dry Fruits", "Yes", "", "", "", "", "", "Show Contents"],
                    &["Inside Packaging", "No"],
                    &["Chocolate", ""],
                ],
            ),
            ..Default::default()
        });
        let got: Vec<_> = r.categories.iter().map(|c| (c.name.as_str(), c.counts_as_item)).collect();
        assert_eq!(got, [("Dry Fruits", true), ("Inside Packaging", false), ("Chocolate", true)]);
        assert_eq!(r.default_detail_mode.as_deref(), Some("Show Contents"));
    }

    #[test]
    fn product_master_by_position_with_margin_normalised() {
        let r = build_import(&ImportInput {
            products: grid(
                8,
                &[
                    &["P001", "Dry Fruits", "Almonds 250g", "Vendor A", "250", "35", "400", "Yes"],
                    &["P002", "Chocolate", "Truffle Box", "Vendor B", "120", "0.4", "200", "No"],
                ],
            ),
            ..Default::default()
        });
        assert_eq!(r.products.len(), 2);
        assert_eq!(
            r.products[0],
            ProductRow {
                code: "P001".into(),
                category_name: Some("Dry Fruits".into()),
                name: "Almonds 250g".into(),
                source: Some("Vendor A".into()),
                cost_price: dec!(250),
                target_margin: dec!(0.35),
                default_sp: dec!(400),
                is_active: true,
                markup_pct: None,
                colors: None,
            }
        );
        assert_eq!(r.products[1].target_margin, dec!(0.4));
        assert!(!r.products[1].is_active);
    }

    #[test]
    fn hamper_summary_keeps_only_input_columns() {
        let r = build_import(&ImportInput {
            hamper_summary: grid(
                16,
                &[&[
                    "H001",
                    "Festive Box",
                    "Diwali",
                    "Approved",
                    "5",
                    "950",
                    "1550",
                    "10",
                    "1395",
                    "1300",
                    "95",
                    "1500",
                    "550",
                    "0.3667",
                    "VIP client",
                    "2025-01-05",
                ]],
            ),
            ..Default::default()
        });
        assert_eq!(
            r.hampers[0],
            HamperRow {
                code: "H001".into(),
                name: "Festive Box".into(),
                collection: Some("Diwali".into()),
                status: "Approved".into(),
                discount_pct: dec!(0.1),
                target_sp: Some(dec!(1300)),
                final_catalogue_sp: Some(dec!(1500)),
                notes: Some("VIP client".into()),
            }
        );
    }

    #[test]
    fn detail_lines_numbered_per_hamper_and_orphans_dropped() {
        let r = build_import(&ImportInput {
            hamper_summary: grid(16, &[&["H001", "Festive Box"]]),
            hamper_details: grid(
                15,
                &[
                    &[
                        "H001",
                        "Festive Box",
                        "",
                        "",
                        "P001",
                        "Dry Fruits",
                        "Almonds",
                        "2",
                        "Vendor A",
                        "250",
                        "500",
                        "35",
                        "400",
                        "800",
                        "",
                    ],
                    &[
                        "H001",
                        "Festive Box",
                        "",
                        "",
                        "P002",
                        "Chocolate",
                        "Truffles",
                        "3",
                        "Vendor B",
                        "120",
                        "360",
                        "40",
                        "200",
                        "600",
                        "",
                    ],
                    &[
                        "H999",
                        "Ghost",
                        "",
                        "",
                        "P001",
                        "Dry Fruits",
                        "Almonds",
                        "1",
                        "",
                        "250",
                        "250",
                        "35",
                        "400",
                        "400",
                        "",
                    ],
                ],
            ),
            ..Default::default()
        });
        assert_eq!(r.hamper_items.len(), 2);
        assert_eq!(r.hamper_items.iter().map(|i| i.line_no).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(
            (r.hamper_items[0].qty, r.hamper_items[0].unit_cp, r.hamper_items[0].unit_sp),
            (dec!(2), dec!(250), dec!(400))
        );
        assert!(r.warnings.iter().any(|w| w.contains("H999")));
    }

    #[test]
    fn mismatched_quote_totals_are_reconciled() {
        let r = build_import(&ImportInput {
            quotation_register: grid(
                20,
                &[&[
                    "LLQT-001",
                    "Quotation",
                    "05/01/2025",
                    "Acme Ltd",
                    "Ravi",
                    "999",
                    "a@b.com",
                    "Diwali",
                    "Combined Order",
                    "1",
                    "10",
                    "9500",
                    "1710",
                    "11210",
                    "Sent",
                    "15 Days",
                    "",
                    "note",
                    "",
                    "",
                ]],
            ),
            quote_details: grid(
                11,
                &[&[
                    "LLQT-001",
                    "Option 1",
                    "H001",
                    "Festive Box",
                    "10",
                    "1000",
                    "0",
                    "1000",
                    "10000",
                    "Show Contents",
                    "Absorb into Box & Packaging",
                ]],
            ),
            ..Default::default()
        });
        let q = &r.quotes[0];
        assert_eq!((q.doc_no.as_str(), q.doc_date.as_deref()), ("LLQT-001", Some("2025-01-05")));
        assert_eq!((q.order_discount, q.adj1, q.gst_rate), (dec!(500), dec!(0), dec!(0.18)));
        assert!(r.warnings.iter().any(|w| w.contains("differs from the sum")));
    }

    #[test]
    fn an_extra_charge_is_recovered() {
        let r = build_import(&ImportInput {
            quotation_register: grid(
                20,
                &[&[
                    "LLQT-002",
                    "Proforma Invoice",
                    "",
                    "Beta Corp",
                    "",
                    "",
                    "",
                    "",
                    "Combined Order",
                    "1",
                    "1",
                    "10500",
                    "",
                    "",
                    "Draft",
                    "",
                    "",
                    "",
                    "",
                    "LLQT-001",
                ]],
            ),
            quote_details: grid(
                11,
                &[&["LLQT-002", "", "H001", "Festive Box", "10", "1000", "0", "1000", "10000", "", ""]],
            ),
            ..Default::default()
        });
        let q = &r.quotes[0];
        assert_eq!(q.doc_type, DocType::ProformaInvoice);
        assert_eq!((q.adj1, q.order_discount), (dec!(500), dec!(0)));
        assert_eq!(q.linked_doc_no.as_deref(), Some("LLQT-001"));
    }

    #[test]
    fn counts_describe_what_will_be_written() {
        let r = build_import(&ImportInput {
            settings: grid(8, &[&["Dry Fruits", "Yes"]]),
            products: grid(8, &[&["P001", "Dry Fruits", "Almonds", "", "250", "35", "400", "Yes"]]),
            hamper_summary: grid(16, &[&["H001", "Festive Box"]]),
            hamper_details: grid(
                15,
                &[&["H001", "", "", "", "P001", "Dry Fruits", "Almonds", "2", "", "250", "", "35", "400", "", ""]],
            ),
            ..Default::default()
        });
        assert_eq!(
            r.counts,
            ImportCounts { categories: 1, products: 1, hampers: 1, hamper_items: 1, quotes: 0, quote_items: 0 }
        );
    }
}
