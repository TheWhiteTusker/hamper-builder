//! Money, percent and amount-in-words display (port of `money-format.ts`).

use crate::js::{self, f};
use rust_decimal::Decimal;

/// Indian digit grouping of a whole number: 1234567 → "12,34,567".
pub(crate) fn group_indian(digits: &str) -> String {
    let len = digits.len();
    if len <= 3 {
        return digits.to_string();
    }
    let (head, last3) = digits.split_at(len - 3);
    let mut parts: Vec<&str> = Vec::new();
    let mut rest = head;
    while rest.len() > 2 {
        let (a, b) = rest.split_at(rest.len() - 2);
        parts.push(b);
        rest = a;
    }
    parts.push(rest);
    parts.reverse();
    format!("{},{last3}", parts.join(","))
}

/// "₹1,23,456.00", as `Intl.NumberFormat("en-IN", { style: "currency" })`
/// printed `round2(v)`. Like Intl, a negative zero keeps its minus sign.
pub fn format_inr(v: Decimal) -> String {
    let x = js::round2(f(v));
    let sign = if x.is_sign_negative() { "-" } else { "" };
    let s = format!("{:.2}", x.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, "00"));
    format!("{sign}₹{}.{frac}", group_indian(int))
}

/// [`format_inr`], or "" for a value not set yet.
pub fn format_money(v: Option<Decimal>) -> String {
    v.map(format_inr).unwrap_or_default()
}

/// A fraction as a percent: 0.3667 → "36.7%". "" for a value not set yet.
pub fn format_pct(v: Option<Decimal>, digits: usize) -> String {
    v.map(|v| format!("{}%", js::to_fixed(f(v) * 100.0, digits))).unwrap_or_default()
}

const ONES: [&str; 20] = [
    "",
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "Six",
    "Seven",
    "Eight",
    "Nine",
    "Ten",
    "Eleven",
    "Twelve",
    "Thirteen",
    "Fourteen",
    "Fifteen",
    "Sixteen",
    "Seventeen",
    "Eighteen",
    "Nineteen",
];
const TENS: [&str; 10] = ["", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety"];

fn below100(n: u64) -> String {
    let at = |list: &[&'static str], i: u64| list.get(i as usize).copied().unwrap_or_default();
    if n < 20 {
        at(&ONES, n).to_string()
    } else {
        format!("{} {}", at(&TENS, n / 10), at(&ONES, n % 10)).trim().to_string()
    }
}

/// Indian grouping: crore, lakh, thousand, hundred, rest.
fn word_groups(mut n: u64) -> Vec<String> {
    let mut parts = Vec::new();
    if n >= 10_000_000 {
        parts.push(format!("{} Crore", word_groups(n / 10_000_000).join(" ")));
        n %= 10_000_000;
    }
    for (size, label) in [(100_000, "Lakh"), (1_000, "Thousand"), (100, "Hundred")] {
        if n >= size {
            parts.push(format!("{} {label}", below100(n / size)));
            n %= size;
        }
    }
    if n > 0 {
        parts.push(below100(n));
    }
    parts
}

fn join_groups(parts: &[String]) -> String {
    match parts.split_last() {
        None => "Zero".to_string(),
        Some((last, [])) => last.clone(),
        Some((last, init)) => format!("{} And {last}", init.join(" ")),
    }
}

/// 70800 → "Seventy Thousand And Eight Hundred Rupees Only", as printed on invoices.
pub fn amount_in_words(amount: Decimal) -> String {
    let total = js::round(f(amount).abs() * 100.0);
    // Saturates past ~1.8e19 paise; no invoice gets near it.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = if total.is_finite() && total > 0.0 { total as u64 } else { 0 };
    let paise = total % 100;
    let rupees = join_groups(&word_groups(total / 100));
    if paise > 0 {
        format!("{rupees} Rupees And {} Paise Only", join_groups(&word_groups(paise)))
    } else {
        format!("{rupees} Rupees Only")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn amount_in_words_follows_the_invoice_wording() {
        assert_eq!(amount_in_words(dec!(70800)), "Seventy Thousand And Eight Hundred Rupees Only");
        assert_eq!(amount_in_words(dec!(105)), "One Hundred And Five Rupees Only");
        assert_eq!(amount_in_words(dec!(0)), "Zero Rupees Only");
        assert_eq!(
            amount_in_words(dec!(123456789.5)),
            "Twelve Crore Thirty Four Lakh Fifty Six Thousand Seven Hundred And Eighty Nine Rupees And Fifty Paise Only"
        );
    }

    #[test]
    fn inr_uses_indian_grouping() {
        assert_eq!(format_inr(dec!(0)), "₹0.00");
        assert_eq!(format_inr(dec!(999)), "₹999.00");
        assert_eq!(format_inr(dec!(1000)), "₹1,000.00");
        assert_eq!(format_inr(dec!(123456.789)), "₹1,23,456.79");
        assert_eq!(format_inr(dec!(12345678)), "₹1,23,45,678.00");
        assert_eq!(format_inr(dec!(-1234.5)), "-₹1,234.50");
        assert_eq!(format_money(None), "");
    }

    #[test]
    fn pct_formats_fractions() {
        assert_eq!(format_pct(Some(dec!(0.36667)), 1), "36.7%");
        assert_eq!(format_pct(Some(dec!(0.5)), 0), "50%");
        assert_eq!(format_pct(None, 1), "");
    }
}
