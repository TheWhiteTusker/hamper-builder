//! Hamper and quote maths (port of `pricing.ts`).
//!
//! The on-device mirror of the `hamper_summary` and `quote_summary` SQL views,
//! used for live totals while someone types. Once a record is saved the views
//! are the authority: if the two ever disagree, the view is right and this
//! file is the bug. Arithmetic follows JS doubles, as the old app's did.

use crate::js::{self, dec, f};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- hampers

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HamperLineInput {
    pub qty: Decimal,
    pub unit_cp: Decimal,
    pub unit_sp: Decimal,
    /// `categoryCountsAsItem()`: unknown categories count, so this defaults to true.
    #[serde(default = "yes")]
    pub counts_as_item: bool,
}

const fn yes() -> bool {
    true
}

impl HamperLineInput {
    pub fn new(qty: Decimal, unit_cp: Decimal, unit_sp: Decimal) -> Self {
        Self { qty, unit_cp, unit_sp, counts_as_item: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HamperOpts {
    pub discount_pct: Decimal,
    pub target_sp: Option<Decimal>,
    pub final_catalogue_sp: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HamperPricing {
    pub total_cp: Decimal,
    pub base_sp: Decimal,
    pub number_of_items: Decimal,
    pub sp_after_discount: Decimal,
    /// None until a Target SP is set.
    pub variance: Option<Decimal>,
    /// None until a Final Catalogue SP is approved.
    pub gross_profit: Option<Decimal>,
    pub final_margin: Option<Decimal>,
}

pub fn price_hamper(lines: &[HamperLineInput], opts: &HamperOpts) -> HamperPricing {
    let discount = f(opts.discount_pct);
    let (mut total_cp, mut base_sp, mut items) = (0.0, 0.0, 0.0);
    for line in lines {
        let qty = f(line.qty);
        total_cp += qty * f(line.unit_cp);
        base_sp += qty * f(line.unit_sp);
        // Inside packaging adds cost but is not one of the "items" advertised.
        if line.counts_as_item {
            items += qty;
        }
    }
    let after_discount = base_sp * (1.0 - discount);
    // Gross profit is measured against the *approved* catalogue price, never
    // the computed one; refreshing prices preserves that override on purpose.
    let final_sp = opts.final_catalogue_sp.map(f);
    let gross_profit = final_sp.map(|sp| sp - total_cp);
    let final_margin = match (gross_profit, final_sp) {
        (Some(gp), Some(sp)) if sp != 0.0 => Some(gp / sp),
        _ => None,
    };
    HamperPricing {
        total_cp: dec(total_cp),
        base_sp: dec(base_sp),
        number_of_items: dec(items),
        sp_after_discount: dec(after_discount),
        variance: opts.target_sp.map(|t| dec(after_discount - f(t))),
        gross_profit: gross_profit.map(dec),
        final_margin: final_margin.map(dec),
    }
}

/// A line's unit price at a margin typed as a whole percent: cost ÷ (1 − margin),
/// rounded up to the next 10. None without a cost, or at 100%+ (no finite price).
pub fn sp_at_margin(cp: Decimal, margin_pct: Decimal) -> Option<Decimal> {
    let (cp, divisor) = (f(cp), 1.0 - f(margin_pct) / 100.0);
    (cp > 0.0 && divisor > 0.0).then(|| dec(js::ceil10(cp / divisor)))
}

/// Margin as a whole percent, (price − cost) ÷ price; None without a price.
pub fn margin_pct_of(cp: Decimal, sp: Decimal) -> Option<Decimal> {
    let (cp, sp) = (f(cp), f(sp));
    (sp > 0.0).then(|| dec(js::round2((sp - cp) / sp * 100.0)))
}

// ----------------------------------------------------------------- quotes

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuoteLineInput {
    pub qty: Decimal,
    pub catalogue_price: Decimal,
    #[serde(default)]
    pub discount_pct: Decimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuoteLinePricing {
    pub final_rate: Decimal,
    pub amount: Decimal,
}

fn line_amounts(line: &QuoteLineInput) -> (f64, f64) {
    let final_rate = f(line.catalogue_price) * (1.0 - f(line.discount_pct));
    (final_rate, f(line.qty) * final_rate)
}

/// The `final_rate` and `amount` generated columns on `quote_items`.
pub fn price_quote_line(line: &QuoteLineInput) -> QuoteLinePricing {
    let (final_rate, amount) = line_amounts(line);
    QuoteLinePricing { final_rate: dec(final_rate), amount: dec(amount) }
}

pub const COMBINED_ORDER: &str = "Combined Order";

/// An option-based quote groups lines by label; a blank label is the first option.
pub fn option_key(label: Option<&str>) -> String {
    match label.map(str::trim) {
        Some(l) if !l.is_empty() => l.to_string(),
        _ => "Option 1".to_string(),
    }
}

#[derive(Debug, PartialEq)]
pub struct OptionGroup<'a, T> {
    pub label: String,
    pub lines: Vec<&'a T>,
}

/// Lines grouped into options, in the order each option first appears.
pub fn group_by_option<'a, T>(lines: &'a [T], label: impl Fn(&T) -> Option<&str>) -> Vec<OptionGroup<'a, T>> {
    let mut groups: Vec<OptionGroup<'a, T>> = Vec::new();
    for line in lines {
        let key = option_key(label(line));
        match groups.iter_mut().find(|g| g.label == key) {
            Some(g) => g.lines.push(line),
            None => groups.push(OptionGroup { label: key, lines: vec![line] }),
        }
    }
    groups
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuoteOpts {
    pub quote_structure: String,
    pub order_discount: Decimal,
    pub adj1: Decimal,
    pub adj2: Decimal,
    pub gst_rate: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotePricing {
    pub subtotal: Decimal,
    pub total_qty: Decimal,
    /// None for option-style quotes: the client picks one, so there is no total.
    pub taxable_value: Option<Decimal>,
    pub gst_amount: Option<Decimal>,
    pub grand_total: Option<Decimal>,
}

pub fn price_quote<'a>(lines: impl IntoIterator<Item = &'a QuoteLineInput>, opts: &QuoteOpts) -> QuotePricing {
    let (mut subtotal, mut total_qty) = (0.0, 0.0);
    for line in lines {
        subtotal += line_amounts(line).1;
        total_qty += f(line.qty);
    }
    // Mirrors =IF(B7="Combined Order", ...) on the sheet's total cells.
    if opts.quote_structure != COMBINED_ORDER {
        return QuotePricing {
            subtotal: dec(subtotal),
            total_qty: dec(total_qty),
            taxable_value: None,
            gst_amount: None,
            grand_total: None,
        };
    }
    let taxable = subtotal - f(opts.order_discount) + f(opts.adj1) + f(opts.adj2);
    let gst = taxable * f(opts.gst_rate);
    QuotePricing {
        subtotal: dec(subtotal),
        total_qty: dec(total_qty),
        taxable_value: Some(dec(taxable)),
        gst_amount: Some(dec(gst)),
        grand_total: Some(dec(taxable + gst)),
    }
}

/// One option of an option-based quote, priced like its own combined order.
/// Order discount and charges belong to a combined order only.
pub fn price_option<'a>(lines: impl IntoIterator<Item = &'a QuoteLineInput>, gst_rate: Decimal) -> QuotePricing {
    price_quote(lines, &QuoteOpts { quote_structure: COMBINED_ORDER.into(), gst_rate, ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numbers::{normalize_pct_str, round2};
    use rust_decimal_macros::dec;

    // A hamper worked by hand, the way it would be on the sheet:
    //   Dry Fruits   2 x 250 CP / 400 SP     CP 500    SP  800   counts
    //   Chocolate    3 x 120 CP / 200 SP     CP 360    SP  600   counts
    //   Inside Box   1 x  90 CP / 150 SP     CP  90    SP  150   does NOT count
    //   discount 10% -> 1395; target 1300 -> +95; final 1500 -> GP 550, 36.67%
    fn lines() -> Vec<HamperLineInput> {
        vec![
            HamperLineInput::new(dec!(2), dec!(250), dec!(400)),
            HamperLineInput::new(dec!(3), dec!(120), dec!(200)),
            HamperLineInput { counts_as_item: false, ..HamperLineInput::new(dec!(1), dec!(90), dec!(150)) },
        ]
    }

    #[test]
    fn hamper_totals_sum_the_line_snapshots() {
        let p = price_hamper(&lines(), &HamperOpts::default());
        assert_eq!(p.total_cp, dec!(950));
        assert_eq!(p.base_sp, dec!(1550));
    }

    #[test]
    fn number_of_items_skips_non_counting_categories() {
        assert_eq!(price_hamper(&lines(), &HamperOpts::default()).number_of_items, dec!(5));
        let one = [HamperLineInput::new(dec!(4), dec!(1), dec!(2))];
        assert_eq!(price_hamper(&one, &HamperOpts::default()).number_of_items, dec!(4));
    }

    #[test]
    fn discount_variance_profit_and_margin() {
        let opts =
            HamperOpts { discount_pct: dec!(0.1), target_sp: Some(dec!(1300)), final_catalogue_sp: Some(dec!(1500)) };
        let p = price_hamper(&lines(), &opts);
        assert_eq!(round2(p.sp_after_discount), dec!(1395));
        assert_eq!(p.variance.map(round2), Some(dec!(95)));
        assert_eq!(p.gross_profit.map(round2), Some(dec!(550)));
        assert_eq!(p.final_margin.map(|m| round2(m * dec!(100))), Some(dec!(36.67)));
    }

    #[test]
    fn profit_and_margin_stay_none_until_a_final_price() {
        let p = price_hamper(&lines(), &HamperOpts { discount_pct: dec!(0.1), ..Default::default() });
        assert_eq!((p.gross_profit, p.final_margin, p.variance), (None, None, None));
        let zero = HamperOpts { final_catalogue_sp: Some(dec!(0)), ..Default::default() };
        assert_eq!(price_hamper(&lines(), &zero).final_margin, None);
    }

    #[test]
    fn gross_profit_uses_the_approved_price() {
        let opts = HamperOpts { discount_pct: dec!(0.5), final_catalogue_sp: Some(dec!(1200)), ..Default::default() };
        assert_eq!(price_hamper(&lines(), &opts).gross_profit, Some(dec!(250)));
    }

    #[test]
    fn quote_line_rate_and_amount() {
        let p =
            price_quote_line(&QuoteLineInput { qty: dec!(10), catalogue_price: dec!(1500), discount_pct: dec!(0.05) });
        assert_eq!(p.final_rate, dec!(1425));
        assert_eq!(p.amount, dec!(14250));
        let blank = QuoteLineInput { qty: dec!(2), catalogue_price: dec!(100), ..Default::default() };
        assert_eq!(price_quote_line(&blank).amount, dec!(200));
    }

    #[test]
    fn combined_order_rolls_up_to_a_grand_total() {
        let lines = [
            QuoteLineInput { qty: dec!(10), catalogue_price: dec!(1500), discount_pct: dec!(0.05) },
            QuoteLineInput { qty: dec!(5), catalogue_price: dec!(2000), ..Default::default() },
        ];
        let opts = QuoteOpts {
            quote_structure: COMBINED_ORDER.into(),
            order_discount: dec!(250),
            adj1: dec!(500),
            adj2: dec!(0),
            gst_rate: dec!(0.18),
        };
        let q = price_quote(&lines, &opts);
        assert_eq!(q.subtotal, dec!(24250));
        assert_eq!(q.total_qty, dec!(15));
        assert_eq!(q.taxable_value, Some(dec!(24500)));
        assert_eq!(q.gst_amount, Some(dec!(4410)));
        assert_eq!(q.grand_total, Some(dec!(28910)));
    }

    #[test]
    fn option_style_quotes_have_no_order_total() {
        let lines = [QuoteLineInput { qty: dec!(1), catalogue_price: dec!(1000), ..Default::default() }];
        let opts = QuoteOpts { quote_structure: "Option Based".into(), gst_rate: dec!(0.18), ..Default::default() };
        let q = price_quote(&lines, &opts);
        assert_eq!(q.subtotal, dec!(1000));
        assert_eq!((q.taxable_value, q.grand_total), (None, None));
    }

    #[test]
    fn option_lines_group_by_label_and_price_alone() {
        let rows = [
            (Some("Option 1"), dec!(10), dec!(500)),
            (Some("Option 2"), dec!(10), dec!(900)),
            (Some(" Option 1 "), dec!(10), dec!(200)),
            (None, dec!(1), dec!(50)),
        ];
        let groups = group_by_option(&rows, |r| r.0);
        let shape: Vec<_> = groups.iter().map(|g| (g.label.as_str(), g.lines.len())).collect();
        assert_eq!(shape, [("Option 1", 3), ("Option 2", 1)]);

        let first: Vec<_> = groups[0]
            .lines
            .iter()
            .map(|r| QuoteLineInput { qty: r.1, catalogue_price: r.2, ..Default::default() })
            .collect();
        let p = price_option(&first, dec!(0.18));
        assert_eq!(p.subtotal, dec!(7050));
        assert_eq!(p.gst_amount, Some(dec!(1269)));
        assert_eq!(p.grand_total, Some(dec!(8319)));
    }

    #[test]
    fn percentages_survive_both_export_shapes() {
        assert_eq!(normalize_pct_str("0.35"), dec!(0.35));
        assert_eq!(normalize_pct_str("35"), dec!(0.35));
        assert_eq!(normalize_pct_str("80.00%"), dec!(0.8));
        assert_eq!(normalize_pct_str(""), dec!(0));
    }

    #[test]
    fn line_margin_and_price_follow_each_other() {
        assert_eq!(sp_at_margin(dec!(100), dec!(35)), Some(dec!(160)));
        assert_eq!(sp_at_margin(dec!(100), dec!(100)), None);
        assert_eq!(sp_at_margin(dec!(0), dec!(35)), None);
        assert_eq!(margin_pct_of(dec!(100), dec!(160)), Some(dec!(37.5)));
        assert_eq!(margin_pct_of(dec!(100), dec!(0)), None);
        // The old app's float quirk, kept: 752 ÷ 0.1 is 7520.000000000002 → 7530.
        assert_eq!(sp_at_margin(dec!(752), dec!(90)), Some(dec!(7530)));
    }
}
