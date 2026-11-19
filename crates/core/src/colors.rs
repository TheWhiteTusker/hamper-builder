//! Product colours, their two-letter codes and swatches (port of `product-colors.ts`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandardColor {
    pub name: &'static str,
    pub code: &'static str,
    pub hex: &'static str,
}

pub const STANDARD_PRODUCT_COLORS: [StandardColor; 3] = [
    StandardColor { name: "Walnut", code: "WL", hex: "#5c4033" },
    StandardColor { name: "Natural", code: "NT", hex: "#e4cdad" },
    StandardColor { name: "Black", code: "BL", hex: "#222222" },
];

/// Swatch for a colour that has no picked hex yet.
pub const FALLBACK_COLOR_HEX: &str = "#94a3b8";

/// Standard colour code for a lower-cased name, code or "… finish".
pub fn color_to_code(key: &str) -> Option<&'static str> {
    match key {
        "walnut" | "wl" | "walnut finish" => Some("WL"),
        "natural" | "nt" | "natural finish" => Some("NT"),
        "black" | "bl" | "black finish" => Some("BL"),
        _ => None,
    }
}

/// Standard colour name for an exact (upper-case) code.
pub fn code_to_color(code: &str) -> Option<&'static str> {
    STANDARD_PRODUCT_COLORS.iter().find(|c| c.code == code).map(|c| c.name)
}

/// Codes pinned by hand where the rule would clash: Black Leather and Brown
/// Leather both derive BR, and Black Leather products already use BR.
fn fixed_code(key: &str) -> Option<&'static str> {
    match key {
        "black leather" => Some("BR"),
        "brown leather" => Some("BN"),
        _ => None,
    }
}

/// Two letters from a name: initials of the first two words, or the first two
/// letters of a single word ("Lights & Candles" → LC, "Decor" → DE).
pub fn derive_category_code(name: &str) -> String {
    const PUNCT: &str = "&/\\#,+()$~%.'\":*?<>{}";
    let spaced: String = name.trim().chars().map(|c| if PUNCT.contains(c) { ' ' } else { c }).collect();
    let words: Vec<&str> = spaced.split_whitespace().collect();
    match words.as_slice() {
        [] => "XX".to_string(),
        [first, second, ..] => {
            let initial = |w: &str| w.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            format!("{}{}", initial(first), initial(second))
        }
        [only] => {
            let clean: String = only.chars().filter(char::is_ascii_alphanumeric).take(2).collect();
            format!("{:X<2}", clean.to_uppercase())
        }
    }
}

/// Two-letter code for any colour. The standard three keep WL/NT/BL; anything
/// added in Settings gets its initials ("Dark Oak" → DO, "Teak" → TE). A clash
/// with a standard code falls back to first + last letter ("Blue" → BE).
pub fn color_code(name: &str) -> String {
    let key = name.trim().to_lowercase();
    if let Some(code) = color_to_code(&key).or_else(|| fixed_code(&key)) {
        return code.to_string();
    }
    let code = derive_category_code(name);
    if !STANDARD_PRODUCT_COLORS.iter().any(|c| c.code == code) {
        return code;
    }
    let letters: Vec<char> = key.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect();
    match (letters.first(), letters.last()) {
        (Some(a), Some(b)) => format!("{a}{b}").to_uppercase(),
        _ => code,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeClash {
    pub name: String,
    pub code: String,
    pub with: String,
}

/// The first colour in `names` whose code another already has. Codes must be unique.
pub fn code_clash<S: AsRef<str>>(names: &[S]) -> Option<CodeClash> {
    let mut by_code: HashMap<String, &str> = HashMap::new();
    for name in names {
        let name = name.as_ref();
        let code = color_code(name);
        if let Some(other) = by_code.get(&code) {
            return Some(CodeClash { name: name.to_string(), code, with: (*other).to_string() });
        }
        by_code.insert(code, name);
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductColor {
    pub name: String,
    pub code: String,
    pub hex: String,
}

/// Colours from Settings with code and swatch: the hex picked in the Master
/// wins, then the standard swatch, then neutral grey.
pub fn resolve_colors<S: AsRef<str>>(names: &[S], hex_by_name: &HashMap<String, String>) -> Vec<ProductColor> {
    names
        .iter()
        .map(|name| {
            let name = name.as_ref();
            let hex = hex_by_name.get(name).cloned().unwrap_or_else(|| {
                STANDARD_PRODUCT_COLORS
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .map_or(FALLBACK_COLOR_HEX, |c| c.hex)
                    .to_string()
            });
            ProductColor { name: name.to_string(), code: color_code(name), hex }
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorTag {
    pub color: Option<String>,
    pub color_code: Option<String>,
}

/// An image's colour tag from a picked name or code; standard colours get their full name.
pub fn color_tag(raw: Option<&str>) -> ColorTag {
    let Some(raw) = raw.filter(|r| !r.is_empty()) else { return ColorTag::default() };
    let code = color_code(raw);
    let color = code_to_color(&code).map_or_else(|| raw.to_string(), str::to_string);
    ColorTag { color: Some(color), color_code: Some(code) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_colors_are_walnut_natural_black() {
        let got: Vec<_> = STANDARD_PRODUCT_COLORS.iter().map(|c| (c.name, c.code)).collect();
        assert_eq!(got, [("Walnut", "WL"), ("Natural", "NT"), ("Black", "BL")]);
    }

    #[test]
    fn derive_category_code_uses_initials() {
        assert_eq!(derive_category_code("Lights & Candles"), "LC");
        assert_eq!(derive_category_code("Box & Packaging"), "BP");
        assert_eq!(derive_category_code("Decor"), "DE");
        assert_eq!(derive_category_code("Utility"), "UT");
        assert_eq!(derive_category_code("A"), "AX");
        assert_eq!(derive_category_code("   "), "XX");
        assert_eq!(derive_category_code(""), "XX");
    }

    #[test]
    fn color_code_keeps_standard_and_derives_added() {
        assert_eq!(color_code("Walnut"), "WL");
        assert_eq!(color_code("NT"), "NT");
        assert_eq!(color_code("Brown Paper"), "BP");
        assert_eq!(color_code("Teak"), "TE");
        assert_eq!(color_code("Blue"), "BE");
        let hex = HashMap::from([("Teak".to_string(), "#b5651d".to_string())]);
        assert_eq!(
            resolve_colors(&["Black", "Teak", "White"], &hex),
            [
                ProductColor { name: "Black".into(), code: "BL".into(), hex: "#222222".into() },
                ProductColor { name: "Teak".into(), code: "TE".into(), hex: "#b5651d".into() },
                ProductColor { name: "White".into(), code: "WH".into(), hex: "#94a3b8".into() },
            ]
        );
    }

    #[test]
    fn codes_are_unique_across_the_live_palette() {
        let live = [
            "Walnut",
            "Natural",
            "Black",
            "Brown Paper",
            "Tan",
            "White Paper",
            "Gold",
            "Jute Brown",
            "Frosted Candle",
            "Neutral",
            "Tan Leather",
            "Brown Leather",
            "Black Leather",
            "Green Leather",
        ];
        assert_eq!(code_clash(&live), None);
        assert_eq!(color_code("Black Leather"), "BR");
        assert_eq!(
            code_clash(&["Brown Leather", "Brown Linen"]),
            Some(CodeClash { name: "Brown Linen".into(), code: "BN".into(), with: "Brown Leather".into() })
        );
    }

    #[test]
    fn color_tag_names_standard_colors() {
        assert_eq!(color_tag(Some("wl")), ColorTag { color: Some("Walnut".into()), color_code: Some("WL".into()) });
        assert_eq!(color_tag(Some("Teak")), ColorTag { color: Some("Teak".into()), color_code: Some("TE".into()) });
        assert_eq!(color_tag(None), ColorTag::default());
        assert_eq!(color_tag(Some("")), ColorTag::default());
    }
}
