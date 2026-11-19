//! JavaScript number semantics, so every figure matches the old app exactly.
//!
//! The TypeScript app works in IEEE-754 doubles, and its results are what
//! the database already holds. To agree with it to the paisa (including where
//! binary floats round a half-paisa down, or push an SP up by ₹10), the maths
//! here runs in `f64` in the same operation order, with `Math.round`,
//! `Math.ceil`, `Number()` and `toFixed` behaving as they do in JS.
//!
//! Values cross the API as [`Decimal`], converted through the shortest
//! round-trip string, so `f64 → Decimal → f64` is lossless and a Decimal holds
//! exactly the digits JS would print.

use rust_decimal::Decimal;
use std::str::FromStr;

/// A Decimal as the double JS would hold for it (correctly rounded).
pub fn f(d: Decimal) -> f64 {
    d.to_string().parse().unwrap_or(0.0)
}

/// An optional Decimal as a double; None is 0, as `num(null)` was.
pub fn fo(d: Option<Decimal>) -> f64 {
    d.map_or(0.0, f)
}

/// A double as the Decimal of its shortest round-trip digits. Non-finite is 0,
/// as `Number.isFinite(n) ? n : 0` made it.
pub fn dec(x: f64) -> Decimal {
    if !x.is_finite() || x == 0.0 {
        return Decimal::ZERO;
    }
    Decimal::from_str(&x.to_string()).or_else(|_| Decimal::from_scientific(&format!("{x:e}"))).unwrap_or(if x > 0.0 {
        Decimal::MAX
    } else {
        Decimal::MIN
    })
}

/// `Math.round`: the nearest integer, ties toward +∞.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let r = x.floor();
    if x - r >= 0.5 { r + 1.0 } else { r }
}

/// `Math.round((n + Number.EPSILON) * 100) / 100`, the old app's `round2`.
pub fn round2(n: f64) -> f64 {
    round((n + f64::EPSILON) * 100.0) / 100.0
}

/// `roundUpToNext10`: the next multiple of 10 at or above `n`; 0 for zero or less.
pub fn ceil10(n: f64) -> f64 {
    if !n.is_finite() || n <= 0.0 { 0.0 } else { (n / 10.0).ceil() * 10.0 }
}

/// `Number(s)` for a string already stripped of whitespace; non-finite is 0.
pub fn number(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let radix = |prefix: &[&str], base| prefix.iter().find_map(|p| s.strip_prefix(p)).map(|d| (d, base));
    let n = match radix(&["0x", "0X"], 16).or_else(|| radix(&["0o", "0O"], 8)).or_else(|| radix(&["0b", "0B"], 2)) {
        Some((digits, base)) => u64::from_str_radix(digits, base).map_or(f64::NAN, |v| v as f64),
        // Rust also reads "inf"/"nan", which JS does not; both end up 0 below.
        None => s.parse::<f64>().unwrap_or(f64::NAN),
    };
    if n.is_finite() { n } else { 0.0 }
}

/// The old app's `num()` of a cell: drop `,`, whitespace, `₹` and `%`, then `Number()`.
pub fn num_str(raw: &str) -> f64 {
    let cleaned: String =
        raw.chars().filter(|c| !matches!(c, ',' | '₹' | '%' | '\u{feff}') && !c.is_whitespace()).collect();
    number(&cleaned)
}

/// `Number.prototype.toFixed`: the exact value rounded to `digits`, ties away from zero.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "NaN".into()
        } else if x > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if x.abs() >= 1e21 {
        return x.to_string();
    }
    // A double below 1e21 has at most ~52 fractional binary digits, so 60
    // decimal places print it exactly and a tie can be seen for what it is.
    let exact = format!("{:.60}", x.abs());
    let (int, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let mut digits_vec: Vec<u8> = int.bytes().chain(frac.bytes().take(digits)).map(|b| b - b'0').collect();
    if frac.as_bytes().get(digits).is_some_and(|b| *b >= b'5') {
        let mut i = digits_vec.len();
        loop {
            if i == 0 {
                digits_vec.insert(0, 1);
                break;
            }
            i -= 1;
            if let Some(d) = digits_vec.get_mut(i) {
                if *d == 9 {
                    *d = 0;
                } else {
                    *d += 1;
                    break;
                }
            }
        }
    }
    let s: String = digits_vec.iter().map(|d| char::from(b'0' + d)).collect();
    let (whole, part) = s.split_at(s.len() - digits);
    let body = if digits == 0 { whole.to_string() } else { format!("{whole}.{part}") };
    let negative = x < 0.0 && body.bytes().any(|b| b.is_ascii_digit() && b != b'0');
    if negative { format!("-{body}") } else { body }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_round_ties_go_up() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
        assert_eq!(round(1.4), 1.0);
    }

    #[test]
    fn round2_matches_js() {
        assert_eq!(round2(55.125), 55.13);
        assert_eq!(round2(1.005), 1.01, "EPSILON lifts small ties");
        // Values checked against Node: these doubles sit just below the tie, so JS rounds down.
        assert_eq!(round2(245.0 * (1.0 + 2.5 / 100.0)), 251.12);
        assert_eq!(round2(10.075), 10.07);
        assert_eq!(round2(4.015), 4.01);
        assert_eq!(round2(1234.565), 1234.57);
    }

    #[test]
    fn decimal_round_trip_is_lossless() {
        for x in [0.1 + 0.2, 7520.000000000002, 36.666666666666664, 1e-7, 123456789.5] {
            assert_eq!(f(dec(x)), x);
        }
        assert_eq!(dec(f64::NAN), Decimal::ZERO);
        assert_eq!(dec(-0.0).to_string(), "0");
    }

    #[test]
    fn number_reads_like_js() {
        assert_eq!(number(""), 0.0);
        assert_eq!(number(".5"), 0.5);
        assert_eq!(number("5."), 5.0);
        assert_eq!(number("1e3"), 1000.0);
        assert_eq!(number("0x10"), 16.0);
        assert_eq!(number("abc"), 0.0);
        assert_eq!(number("Infinity"), 0.0);
        assert_eq!(num_str("₹ 1,250.50"), 1250.5);
    }

    #[test]
    fn to_fixed_matches_js() {
        assert_eq!(to_fixed(0.125, 2), "0.13", "an exact tie goes up");
        assert_eq!(to_fixed(1.005, 2), "1.00", "1.005 is really 1.00499…");
        assert_eq!(to_fixed(36.666666666666664, 1), "36.7");
        assert_eq!(to_fixed(9.96, 1), "10.0");
        assert_eq!(to_fixed(-0.04, 1), "0.0");
        assert_eq!(to_fixed(-1.25, 1), "-1.3");
        assert_eq!(to_fixed(50.0, 0), "50");
    }
}
