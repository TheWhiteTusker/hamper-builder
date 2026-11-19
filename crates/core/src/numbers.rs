//! Number coercion shared by pricing, costing and import (port of `numbers.ts`).
//!
//! Values are [`Decimal`] at the API; the arithmetic follows JS doubles (see
//! [`crate::js`]) so every figure matches the old app exactly.

use crate::js::{self, dec, f};
use rust_decimal::Decimal;

/// Money rounded to paise, as the old app's `round2` did it.
pub fn round2(n: Decimal) -> Decimal {
    dec(js::round2(f(n)))
}

/// `Math.round`: nearest whole number, ties toward +∞.
pub fn round0(n: Decimal) -> Decimal {
    dec(js::round(f(n)))
}

/// A spreadsheet or form cell as a number: blank or non-numeric is 0, the way
/// the sheet's `Number(x)||0` did. Thousands separators, spaces, `₹` and a
/// trailing `%` are dropped (scale percentages with [`normalize_pct`]).
pub fn parse_num(raw: &str) -> Decimal {
    dec(js::num_str(raw))
}

/// Percentages are stored as fractions (0.35 = 35%). Exports sometimes carry a
/// whole percent instead, so anything above 1 is taken as one.
pub fn normalize_pct(n: Decimal) -> Decimal {
    let x = f(n);
    dec(if x > 1.0 { x / 100.0 } else { x })
}

/// [`normalize_pct`] of a raw cell (`"80.00%"`, `"35"`, `"0.35"`).
pub fn normalize_pct_str(raw: &str) -> Decimal {
    normalize_pct(parse_num(raw))
}

/// Rounds a selling price up to the next multiple of 10; 0 for zero or less.
/// 271.50 → 280, 270 → 270, 270.01 → 280.
pub fn round_up_to_next10(n: Decimal) -> Decimal {
    dec(js::ceil10(f(n)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn up10(s: &str) -> Decimal {
        round_up_to_next10(parse_num(s))
    }

    #[test]
    fn round_up_to_next10_rounds_up() {
        assert_eq!(up10("271.5"), dec!(280));
        assert_eq!(up10("271.50"), dec!(280));
        for (input, want) in [("270", 270), ("280", 280), ("10", 10), ("100", 100)] {
            assert_eq!(up10(input), Decimal::from(want), "{input}");
        }
        for (input, want) in [("270.01", 280), ("270.0001", 280), ("0.01", 10), ("1", 10), ("9.99", 10)] {
            assert_eq!(up10(input), Decimal::from(want), "{input}");
        }
        assert_eq!(up10("0"), dec!(0));
        assert_eq!(up10("-5"), dec!(0));
        assert_eq!(up10(""), dec!(0));
        assert_eq!(up10("₹ 271.50"), dec!(280));
        assert_eq!(up10("1,271.50"), dec!(1280));
    }

    #[test]
    fn round2_rounds_to_paise() {
        assert_eq!(round2(dec!(10.123)), dec!(10.12));
        assert_eq!(round2(dec!(10.126)), dec!(10.13));
        assert_eq!(round2(dec!(10)), dec!(10));
        assert_eq!(round2(dec!(55.125)), dec!(55.13));
    }

    #[test]
    fn parse_num_strips_symbols() {
        assert_eq!(parse_num("100"), dec!(100));
        assert_eq!(parse_num("₹ 1,250.50"), dec!(1250.5));
        assert_eq!(parse_num(""), dec!(0));
        assert_eq!(parse_num("abc"), dec!(0));
        assert_eq!(parse_num(".5"), dec!(0.5));
        assert_eq!(parse_num("5."), dec!(5));
        assert_eq!(parse_num("+5"), dec!(5));
        assert_eq!(parse_num("-.5"), dec!(-0.5));
        assert_eq!(parse_num("1e3"), dec!(1000));
    }

    #[test]
    fn normalize_pct_handles_both_export_shapes() {
        assert_eq!(normalize_pct(dec!(0.35)), dec!(0.35));
        assert_eq!(normalize_pct(dec!(35)), dec!(0.35));
        assert_eq!(normalize_pct_str("50%"), dec!(0.5));
        assert_eq!(normalize_pct_str("80.00%"), dec!(0.8));
        assert_eq!(normalize_pct_str(""), dec!(0));
    }
}
