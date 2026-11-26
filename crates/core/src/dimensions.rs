//! Units and dimension-to-area conversion for cost lines (port of `dimensions.ts`).
//! Arithmetic follows JS doubles in the old app's operation order.

use crate::js::{self, dec, f};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The unit a line's length and breadth are entered in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DimUnit {
    #[default]
    Inch,
    Mm,
    Cm,
}

impl DimUnit {
    /// Anything other than `mm` or `cm` (including blank) is inches, as in the TS.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("mm") => Self::Mm,
            Some("cm") => Self::Cm,
            _ => Self::Inch,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inch => "inch",
            Self::Mm => "mm",
            Self::Cm => "cm",
        }
    }
}

pub const COMMON_UNITS: [&str; 15] = [
    "inch",
    "mm",
    "cm",
    "feet",
    "meter",
    "sq ft",
    "sq inch",
    "sq mm",
    "sq cm",
    "running ft",
    "piece",
    "kg",
    "set",
    "min",
    "hour",
];

pub(crate) const LINEAR_UNITS: [&str; 10] = ["feet", "ft", "running ft", "rft", "inch", "in", "mm", "cm", "meter", "m"];
pub(crate) const AREA_UNITS: [&str; 11] =
    ["sq ft", "sqft", "sft", "sq inch", "sqinch", "sq in", "sqin", "sq mm", "sqmm", "sq cm", "sqcm"];

fn to_inches(v: f64, unit: DimUnit) -> f64 {
    match unit {
        DimUnit::Inch => v,
        DimUnit::Mm => v / 25.4,
        DimUnit::Cm => v / 2.54,
    }
}

/// [`dimension_area`] on doubles, for costing to chain without conversions.
pub(crate) fn dimension_area_f(l: f64, b: f64, dim: DimUnit, target_unit: &str) -> f64 {
    let unit = target_unit.trim().to_lowercase();
    let unit = unit.as_str();

    if LINEAR_UNITS.contains(&unit) {
        if l <= 0.0 {
            return 1.0;
        }
        return match unit {
            "feet" | "ft" | "running ft" | "rft" => to_inches(l, dim) / 12.0,
            "inch" | "in" => to_inches(l, dim),
            "mm" => match dim {
                DimUnit::Mm => l,
                DimUnit::Cm => l * 10.0,
                DimUnit::Inch => l * 25.4,
            },
            "cm" => match dim {
                DimUnit::Cm => l,
                DimUnit::Mm => l / 10.0,
                DimUnit::Inch => l * 2.54,
            },
            // "meter" | "m"
            _ => match dim {
                DimUnit::Cm => l / 100.0,
                DimUnit::Mm => l / 1000.0,
                DimUnit::Inch => (l * 2.54) / 100.0,
            },
        };
    }

    if l <= 0.0 || b <= 0.0 {
        return 1.0;
    }
    let (li, bi) = (to_inches(l, dim), to_inches(b, dim));
    let side_mm = |v: f64, inches: f64| match dim {
        DimUnit::Mm => v,
        DimUnit::Cm => v * 10.0,
        DimUnit::Inch => inches * 25.4,
    };
    let side_cm = |v: f64, inches: f64| match dim {
        DimUnit::Cm => v,
        DimUnit::Mm => v / 10.0,
        DimUnit::Inch => inches * 2.54,
    };
    match unit {
        "sq ft" | "sqft" | "sft" => (li * bi) / 144.0,
        "sq inch" | "sqinch" | "sq in" | "sqin" => li * bi,
        "sq mm" | "sqmm" => side_mm(l, li) * side_mm(b, bi),
        "sq cm" | "sqcm" => side_cm(l, li) * side_cm(b, bi),
        // Per piece / each / nos: dimensions describe the item, they don't multiply.
        _ => 1.0,
    }
}

/// The quantity a rate multiplies: a length in the rate's linear unit, an area
/// in its area unit, or 1 when the unit is per piece or a dimension is missing.
pub fn dimension_area(length: Decimal, breadth: Decimal, dim: DimUnit, target_unit: &str) -> Decimal {
    dec(dimension_area_f(f(length), f(breadth), dim, target_unit))
}

/// [`dimension_area`] rounded to 2 decimals, for display.
pub fn calculate_dimension_area(length: Decimal, breadth: Decimal, dim: DimUnit, target_unit: &str) -> Decimal {
    dec(js::round2(dimension_area_f(f(length), f(breadth), dim, target_unit)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numbers::round0;
    use DimUnit::{Cm, Inch, Mm};
    use rust_decimal_macros::dec;

    fn area(l: Decimal, b: Decimal, dim: DimUnit, unit: &str) -> Decimal {
        calculate_dimension_area(l, b, dim, unit)
    }

    #[test]
    fn inches_to_sq_ft() {
        assert_eq!(area(dec!(12), dec!(12), Inch, "sq ft"), dec!(1));
        assert_eq!(area(dec!(24), dec!(12), Inch, "sq ft"), dec!(2));
        assert_eq!(area(dec!(6), dec!(6), Inch, "sq ft"), dec!(0.25));
        assert_eq!(round0(area(dec!(304.8), dec!(304.8), Mm, "sq ft")), dec!(1));
    }

    #[test]
    fn linear_feet() {
        assert_eq!(area(dec!(24), dec!(0), Inch, "feet"), dec!(2));
        assert_eq!(area(dec!(36), dec!(12), Inch, "feet"), dec!(3));
        assert_eq!(area(dec!(304.8), dec!(0), Mm, "feet"), dec!(1));
        assert_eq!(area(dec!(30.48), dec!(0), Cm, "feet"), dec!(1));
    }

    #[test]
    fn linear_inch_mm_cm() {
        assert_eq!(area(dec!(12), dec!(0), Inch, "inch"), dec!(12));
        assert_eq!(area(dec!(50.8), dec!(0), Mm, "inch"), dec!(2));
        assert_eq!(area(dec!(500), dec!(0), Mm, "mm"), dec!(500));
        assert_eq!(area(dec!(50), dec!(0), Cm, "mm"), dec!(500));
        assert_eq!(area(dec!(2), dec!(0), Inch, "mm"), dec!(50.8));
        assert_eq!(area(dec!(25), dec!(0), Cm, "cm"), dec!(25));
        assert_eq!(area(dec!(100), dec!(0), Mm, "cm"), dec!(10));
        assert_eq!(area(dec!(2), dec!(0), Inch, "cm"), dec!(5.08));
    }

    #[test]
    fn square_units() {
        assert_eq!(area(dec!(10), dec!(5), Inch, "sq inch"), dec!(50));
        assert_eq!(area(dec!(25.4), dec!(25.4), Mm, "sq inch"), dec!(1));
        assert_eq!(area(dec!(100), dec!(200), Mm, "sq mm"), dec!(20000));
        assert_eq!(area(dec!(10), dec!(20), Cm, "sq mm"), dec!(20000));
        assert_eq!(area(dec!(10), dec!(20), Cm, "sq cm"), dec!(200));
        assert_eq!(area(dec!(100), dec!(200), Mm, "sq cm"), dec!(200));
        assert_eq!(area(dec!(2), dec!(4), Inch, "sq cm"), dec!(51.61));
    }

    #[test]
    fn per_piece_and_missing_dimensions_are_one() {
        assert_eq!(area(dec!(10), dec!(5), Inch, "piece"), dec!(1));
        assert_eq!(area(dec!(10), dec!(0), Inch, "sq ft"), dec!(1));
        assert_eq!(area(dec!(0), dec!(0), Inch, "feet"), dec!(1));
    }
}
