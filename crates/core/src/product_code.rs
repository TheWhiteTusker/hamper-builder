//! Product codes: `CATEGORY/SERIAL/COLOR`, e.g. `LC/0001/WL` (port of `product-code.ts`).

use crate::colors::{code_to_color, color_code, color_to_code};
use serde::{Deserialize, Serialize};

fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Formats `CATEGORY/SERIAL/COLOR`. The serial is padded to 4 digits (minimum 1);
/// the colour may be a name or code, and blank means Walnut.
pub fn format_product_code(category_code: &str, serial: &str, color: &str) -> String {
    let cat = if category_code.is_empty() { "XX" } else { category_code };
    let cat: String = cat.trim().to_uppercase().chars().filter(char::is_ascii_alphanumeric).collect();
    let n = digits(serial).parse::<u64>().unwrap_or(0).max(1);
    let col = if color.trim().is_empty() { "WL".to_string() } else { color_code(color) };
    format!("{cat}/{n:04}/{col}")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCode {
    pub category_code: String,
    pub serial: String,
    pub color_code: String,
    pub color_name: String,
    /// Strictly `[A-Z0-9]+/\d{4}/[A-Z0-9]{2}`.
    pub is_valid: bool,
}

fn is_upper_alnum(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn name_of(code: &str) -> String {
    code_to_color(code).map_or_else(|| code.to_string(), str::to_string)
}

/// Parses a code; also accepts loose `CAT/12/Walnut` and legacy `LC010` shapes (not valid).
pub fn parse_product_code(code: &str) -> ParsedCode {
    let clean = code.trim().to_uppercase();
    let parts: Vec<&str> = clean.split('/').collect();

    if let [cat, serial, col] = parts.as_slice()
        && is_upper_alnum(cat)
        && is_digits(serial)
        && is_upper_alnum(col)
    {
        if serial.len() == 4 && col.len() == 2 {
            return ParsedCode {
                category_code: (*cat).to_string(),
                serial: (*serial).to_string(),
                color_code: (*col).to_string(),
                color_name: name_of(col),
                is_valid: true,
            };
        }
        let col_code =
            color_to_code(&col.to_lowercase()).map(str::to_string).unwrap_or_else(|| col.chars().take(2).collect());
        return ParsedCode {
            category_code: (*cat).to_string(),
            serial: format!("{serial:0>4}"),
            color_name: name_of(&col_code),
            color_code: col_code,
            is_valid: false,
        };
    }

    // Legacy: letters then digits, like LC010.
    let split = clean.find(|c: char| c.is_ascii_digit()).unwrap_or(clean.len());
    let (letters, nums) = clean.split_at(split);
    if !letters.is_empty() && letters.chars().all(|c| c.is_ascii_uppercase()) && is_digits(nums) {
        return ParsedCode {
            category_code: letters.to_string(),
            serial: format!("{nums:0>4}"),
            color_code: "WL".into(),
            color_name: "Walnut".into(),
            is_valid: false,
        };
    }

    ParsedCode {
        category_code: String::new(),
        serial: "0001".into(),
        color_code: "WL".into(),
        color_name: "Walnut".into(),
        is_valid: false,
    }
}

/// The next 4-digit serial for a category, after the highest among `existing`.
pub fn next_serial_for_category<S: AsRef<str>>(category_code: &str, existing: &[S]) -> String {
    let cat = category_code.trim().to_uppercase();
    if cat.is_empty() {
        return "0001".into();
    }
    let max = existing
        .iter()
        .map(AsRef::as_ref)
        .filter(|c| !c.is_empty())
        .map(parse_product_code)
        .filter(|p| p.category_code == cat)
        .filter_map(|p| p.serial.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("{:04}", max + 1)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorCodeOf {
    pub color: String,
    pub code: String,
}

/// One code per colour, sharing category and serial: `LC/0001/WL` + [Walnut, Black]
/// → `LC/0001/WL`, `LC/0001/BL`.
pub fn codes_for_colors<S: AsRef<str>>(code: &str, colors: &[S]) -> Vec<ColorCodeOf> {
    let parsed = parse_product_code(code);
    colors
        .iter()
        .map(|color| {
            let color = color.as_ref();
            let code = if parsed.category_code.is_empty() {
                format!("{}/{}", code.trim().to_uppercase(), color_code(color))
            } else {
                format_product_code(&parsed.category_code, &parsed.serial, color)
            };
            ColorCodeOf { color: color.to_string(), code }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_category_serial_color() {
        assert_eq!(format_product_code("LC", "1", "WL"), "LC/0001/WL");
        assert_eq!(format_product_code("lc", "1", "natural"), "LC/0001/NT");
        assert_eq!(format_product_code("ED", "42", "black"), "ED/0042/BL");
        assert_eq!(format_product_code("DC", "0123", "BL"), "DC/0123/BL");
        assert_eq!(format_product_code("LC", "1", "Brown Paper"), "LC/0001/BP");
        assert_eq!(format_product_code("", "0", ""), "XX/0001/WL");
    }

    #[test]
    fn parses_valid_codes() {
        let p = parse_product_code("LC/0001/WL");
        assert!(p.is_valid);
        assert_eq!((p.category_code.as_str(), p.serial.as_str()), ("LC", "0001"));
        assert_eq!((p.color_code.as_str(), p.color_name.as_str()), ("WL", "Walnut"));
        let p = parse_product_code("ED/0025/NT");
        assert_eq!((p.is_valid, p.color_name.as_str()), (true, "Natural"));
        let p = parse_product_code("DC/0099/BL");
        assert_eq!((p.is_valid, p.color_name.as_str()), (true, "Black"));
        assert!(parse_product_code("LC/0001/BP").is_valid);
    }

    #[test]
    fn parses_loose_and_legacy_codes_as_invalid() {
        let p = parse_product_code("LC010");
        assert!(!p.is_valid);
        assert_eq!((p.category_code.as_str(), p.serial.as_str()), ("LC", "0010"));
        let p = parse_product_code("lc/12/walnut");
        assert!(!p.is_valid);
        assert_eq!((p.serial.as_str(), p.color_code.as_str(), p.color_name.as_str()), ("0012", "WL", "Walnut"));
        let p = parse_product_code("??");
        assert_eq!((p.category_code.as_str(), p.is_valid), ("", false));
    }

    #[test]
    fn next_serial_follows_the_highest() {
        let existing = ["LC001", "LC002", "LC010", "LC/0001/WL", "ED001"];
        assert_eq!(next_serial_for_category("LC", &existing), "0011");
        assert_eq!(next_serial_for_category("ED", &existing), "0002");
        assert_eq!(next_serial_for_category("DC", &existing), "0001");
    }

    #[test]
    fn codes_for_colors_share_category_and_serial() {
        let got = codes_for_colors("LC/0007/WL", &["Walnut", "Natural", "Black"]);
        let codes: Vec<_> = got.iter().map(|c| (c.color.as_str(), c.code.as_str())).collect();
        assert_eq!(codes, [("Walnut", "LC/0007/WL"), ("Natural", "LC/0007/NT"), ("Black", "LC/0007/BL")]);
        assert_eq!(codes_for_colors("", &["Black"]), [ColorCodeOf { color: "Black".into(), code: "/BL".into() }]);
        let leather = codes_for_colors("AC/0002/BR", &["Black Leather", "Brown Leather"]);
        let codes: Vec<_> = leather.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["AC/0002/BR", "AC/0002/BN"]);
    }
}
