//! MRP caps for bought-out items (port of `mrp.ts`), on JS doubles like the old app.

use crate::costing::{ProductCostLine, StageOverheads, line_cost_f, overhead_factor};
use crate::js::{self, dec, f};
use rust_decimal::Decimal;

pub(crate) fn mrp_limit_f(
    lines: &[ProductCostLine],
    overheads: Option<&StageOverheads>,
    total_cost: f64,
    gst_rate: f64,
) -> Option<f64> {
    let (mut ceiling, mut mrp_cost) = (0.0, 0.0);
    for l in lines {
        let mrp = l.mrp.map_or(0.0, f);
        if mrp <= 0.0 {
            continue;
        }
        ceiling += mrp * l.qty_f();
        mrp_cost += line_cost_f(l).1 * overhead_factor(overheads, &l.stage());
    }
    if ceiling == 0.0 || ceiling.is_nan() {
        return None;
    }
    let share = if total_cost > 0.0 { mrp_cost / total_cost } else { 1.0 };
    // A free MRP item takes none of the price, so it caps nothing.
    if share <= 0.0 || share.is_nan() {
        return None;
    }
    Some(js::round2(ceiling / (1.0 + gst_rate) / share).floor())
}

/// The highest SP (before GST) the MRPs printed on bought-out items allow, in
/// whole rupees; None when no line has an MRP. MRP includes GST, so the items
/// may sell for MRP × qty ÷ (1 + GST). In a product with other costs too, the
/// MRP items take their share of the SP by cost, and only that share is capped.
pub fn mrp_limit(
    lines: &[ProductCostLine],
    overheads: Option<&StageOverheads>,
    total_cost: Decimal,
    gst_rate: Decimal,
) -> Option<Decimal> {
    mrp_limit_f(lines, overheads, f(total_cost), f(gst_rate)).map(dec)
}

/// An SP rounded up to the next 10, unless that would take an SP within the
/// MRP limit over it. `Math.round` absorbs paise of drift from a markup %
/// saved to 2 decimals.
pub fn round_sp(sp: Decimal, limit: Option<Decimal>) -> Decimal {
    let sp = f(sp);
    let rounded = js::ceil10(sp);
    dec(match limit.map(f) {
        Some(limit) if js::round(sp) <= limit && rounded > limit => limit,
        _ => rounded,
    })
}

/// The margin on one bought-out item sold at its MRP (less GST); None without an MRP or qty.
pub fn mrp_margin(line: &ProductCostLine, gst_rate: Decimal) -> Option<Decimal> {
    let qty = line.qty_f();
    let mrp = line.mrp.map_or(0.0, f);
    if mrp <= 0.0 || qty <= 0.0 {
        return None;
    }
    let max_sp = mrp / (1.0 + f(gst_rate));
    Some(dec((max_sp - line_cost_f(line).1 / qty) / max_sp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numbers::round0;
    use rust_decimal_macros::dec;

    pub(crate) fn piece(rate: Decimal, mrp: Option<Decimal>) -> ProductCostLine {
        ProductCostLine {
            stage_code: "bought_out".into(),
            unit: "piece".into(),
            rate,
            qty: Some(dec!(1)),
            mrp,
            ..Default::default()
        }
    }

    #[test]
    fn limit_keeps_sp_plus_gst_within_mrp() {
        let none = StageOverheads::new();
        let o = Some(&none);
        assert_eq!(mrp_limit(&[piece(dec!(300), Some(dec!(499)))], o, dec!(300), dec!(0.18)), Some(dec!(422)));
        let two = ProductCostLine { qty: Some(dec!(2)), ..piece(dec!(300), Some(dec!(499))) };
        assert_eq!(mrp_limit(&[two], o, dec!(600), dec!(0.18)), Some(dec!(845)));
        assert_eq!(mrp_limit(&[piece(dec!(300), None)], o, dec!(300), dec!(0.18)), None);
        assert_eq!(mrp_limit(&[piece(dec!(800), Some(dec!(1180)))], o, dec!(800), dec!(0.18)), Some(dec!(1000)));
    }

    #[test]
    fn limit_caps_only_the_mrp_share_of_a_mixed_product() {
        let none = StageOverheads::new();
        let o = Some(&none);
        let bx = ProductCostLine { stage_code: "material".into(), ..piece(dec!(200), None) };
        assert_eq!(mrp_limit(&[bx.clone(), piece(dec!(300), Some(dec!(400)))], o, dec!(500), dec!(0)), Some(dec!(666)));
        assert_eq!(mrp_limit(&[bx, piece(dec!(0), Some(dec!(400)))], o, dec!(200), dec!(0)), None);
        assert_eq!(mrp_limit(&[piece(dec!(0), Some(dec!(400)))], o, dec!(0), dec!(0)), Some(dec!(400)));
    }

    #[test]
    fn round_sp_never_rounds_within_limit_over_it() {
        assert_eq!(round_sp(dec!(415), Some(dec!(422))), dec!(420));
        assert_eq!(round_sp(dec!(421), Some(dec!(422))), dec!(422));
        assert_eq!(round_sp(dec!(422.003), Some(dec!(422))), dec!(422));
        assert_eq!(round_sp(dec!(423), Some(dec!(422))), dec!(430));
        assert_eq!(round_sp(dec!(421), None), dec!(430));
    }

    #[test]
    fn margin_at_mrp_less_gst() {
        let m = mrp_margin(&piece(dec!(300), Some(dec!(499))), dec!(0.18)).map(|m| round0(m * dec!(100)));
        assert_eq!(m, Some(dec!(29)));
        assert_eq!(mrp_margin(&piece(dec!(300), None), dec!(0.18)), None);
    }
}
