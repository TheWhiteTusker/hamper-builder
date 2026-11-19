//! Forgiving number deserializers for rows from PostgREST and `jsonb` settings.
//!
//! Postgres `numeric` arrives as a JSON number, but jsonb written by the old
//! app or the spreadsheet import can hold `"25"`, `""` or `null` where a number
//! belongs. These read all of them the way the TS `num()` did instead of
//! failing the whole row.

use crate::numbers::parse_num;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

struct NumVisitor;

impl<'de> Visitor<'de> for NumVisitor {
    type Value = Option<Decimal>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number, numeric string or null")
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Some(Decimal::from(v)))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(Decimal::from(v)))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        // Display gives the shortest string that round-trips, so 0.1 stays 0.1.
        Ok(v.is_finite().then(|| Decimal::from_str(&v.to_string()).unwrap_or_default()))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok((!v.trim().is_empty()).then(|| parse_num(v)))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Some(if v { Decimal::ONE } else { Decimal::ZERO }))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(NumVisitor)
    }
}

/// `number | "numeric string" | "" | null` → Decimal (blank and null are 0).
pub fn decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
    Ok(d.deserialize_any(NumVisitor)?.unwrap_or_default())
}

/// `number | "numeric string"` → Some; `"" | null` → None.
pub fn opt_decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Decimal>, D::Error> {
    d.deserialize_any(NumVisitor)
}

/// A jsonb object of numbers such as `{ "material": 10, "misc": "25" }`.
pub fn decimal_map<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, Decimal>, D::Error> {
    #[derive(serde::Deserialize)]
    struct Lenient(#[serde(deserialize_with = "decimal")] Decimal);
    let raw: Option<BTreeMap<String, Lenient>> = serde::Deserialize::deserialize(d)?;
    Ok(raw.unwrap_or_default().into_iter().map(|(k, v)| (k, v.0)).collect())
}

/// `null` as the type's default, for non-optional text columns.
pub fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    struct Row {
        #[serde(deserialize_with = "super::decimal")]
        a: Decimal,
        #[serde(default, deserialize_with = "super::opt_decimal")]
        b: Option<Decimal>,
        #[serde(default, deserialize_with = "super::decimal_map")]
        m: BTreeMap<String, Decimal>,
    }

    #[test]
    fn reads_numbers_strings_and_nulls() -> Result<(), serde_json::Error> {
        let r: Row = serde_json::from_str(r#"{"a": 0.1, "b": "25", "m": {"x": "10", "y": 2.5, "z": ""}}"#)?;
        assert_eq!(r.a, dec!(0.1));
        assert_eq!(r.b, Some(dec!(25)));
        assert_eq!(r.m.get("x"), Some(&dec!(10)));
        assert_eq!(r.m.get("y"), Some(&dec!(2.5)));
        assert_eq!(r.m.get("z"), Some(&dec!(0)));
        let r: Row = serde_json::from_str(r#"{"a": null, "b": null, "m": null}"#)?;
        assert_eq!(r.a, dec!(0));
        assert_eq!(r.b, None);
        assert!(r.m.is_empty());
        let r: Row = serde_json::from_str(r#"{"a": "₹1,200", "b": ""}"#)?;
        assert_eq!(r.a, dec!(1200));
        assert_eq!(r.b, None);
        Ok(())
    }
}
