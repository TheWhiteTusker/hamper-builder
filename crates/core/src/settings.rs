//! Everything in `app_settings`, resolved into one object (port of `settings.ts`).

use crate::lenient;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompanySettings {
    pub name: String,
    /// Registered entity printed on proforma invoices, e.g. "Palm Length LLP".
    pub legal_name: String,
    pub address: String,
    pub gstin: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub bank_account_name: String,
    pub bank_name: String,
    pub bank_account_no: String,
    pub bank_ifsc: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocPrefixes {
    pub hamper: String,
    pub quotation: String,
    pub proforma_invoice: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub company: CompanySettings,
    #[serde(deserialize_with = "lenient::decimal")]
    pub gst_rate: Decimal,
    pub doc_prefixes: DocPrefixes,
    pub hamper_statuses: Vec<String>,
    pub quote_statuses: Vec<String>,
    pub quote_structures: Vec<String>,
    pub validity_options: Vec<String>,
    pub collections: Vec<String>,
    pub sources: Vec<String>,
    pub detail_modes: Vec<String>,
    pub default_detail_mode: String,
    pub packaging_treatments: Vec<String>,
    pub default_packaging_treatment: String,
    pub default_validity: String,
    pub quote_terms: String,
    pub pi_terms: String,
    pub product_colors: Vec<String>,
    /// Swatch per product colour name, picked in the Rates & Hierarchy Master.
    pub color_hex: BTreeMap<String, String>,
    /// Oldest app build allowed to write; older builds are told to update.
    /// Not in the old app's seed, so it is absent until the first breaking migration.
    #[serde(default)]
    pub min_app_version: Option<String>,
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

impl Default for Settings {
    /// Used when a key is missing from `app_settings`; matches `0003_seed.sql`.
    fn default() -> Self {
        Self {
            company: CompanySettings { name: "Lattice Lane".into(), ..Default::default() },
            gst_rate: dec!(0.18),
            doc_prefixes: DocPrefixes {
                hamper: "H".into(),
                quotation: "LLQT-".into(),
                proforma_invoice: "LLPI-".into(),
            },
            hamper_statuses: strings(&["Draft", "Approved", "Active", "Discontinued"]),
            quote_statuses: strings(&["Draft", "Sent", "Negotiation", "Won", "Lost", "Expired"]),
            quote_structures: strings(&["Combined Order", "Option Based"]),
            validity_options: strings(&["7 Days", "15 Days", "30 Days"]),
            collections: Vec::new(),
            sources: strings(&["In-house", "Outsourced", "Hybrid"]),
            detail_modes: strings(&["Show Contents", "Summary Only", "Hide Contents"]),
            default_detail_mode: "Show Contents".into(),
            packaging_treatments: strings(&["Absorb into Box & Packaging", "Charge Separately", "Show as Line Item"]),
            default_packaging_treatment: "Absorb into Box & Packaging".into(),
            default_validity: "15 Days".into(),
            quote_terms: String::new(),
            pi_terms: String::new(),
            product_colors: strings(&["Walnut", "Natural", "Black"]),
            color_hex: BTreeMap::new(),
            min_app_version: None,
        }
    }
}

/// One `app_settings` row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingRow {
    pub key: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedSettings {
    pub settings: Settings,
    /// Keys whose stored value had the wrong shape and fell back to the default.
    pub rejected: Vec<String>,
}

/// Overlays the stored rows on the defaults. `company` merges a level deeper,
/// so a company saved before a field existed still gets that field's default.
/// A key whose value has the wrong shape keeps its default and is reported,
/// instead of breaking every screen that reads settings.
pub fn resolve_settings(rows: &[SettingRow]) -> LoadedSettings {
    let defaults = Settings::default();
    let mut merged = match serde_json::to_value(&defaults) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    let mut rejected = Vec::new();

    for row in rows {
        let mut value = row.value.clone();
        if row.key == "company"
            && let (Some(Value::Object(base)), Value::Object(stored)) = (merged.get("company"), &value)
        {
            let mut company = base.clone();
            company.extend(stored.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())));
            value = Value::Object(company);
        }
        let mut candidate = merged.clone();
        candidate.insert(row.key.clone(), value);
        if serde_json::from_value::<Settings>(Value::Object(candidate.clone())).is_ok() {
            merged = candidate;
        } else {
            rejected.push(row.key.clone());
        }
    }

    let settings = serde_json::from_value(Value::Object(merged)).unwrap_or(defaults);
    LoadedSettings { settings, rejected }
}

/// Adds values found in imported data to a pick-list (case-insensitively new
/// ones only), so the strings the spreadsheet used survive the move.
pub fn merge_list<S: AsRef<str>>(existing: &[String], found: &[Option<S>]) -> Vec<String> {
    let mut merged = existing.to_vec();
    let mut seen: Vec<String> = existing.iter().map(|v| v.to_lowercase()).collect();
    for raw in found.iter().flatten() {
        let value = raw.as_ref().trim();
        if value.is_empty() || seen.contains(&value.to_lowercase()) {
            continue;
        }
        seen.push(value.to_lowercase());
        merged.push(value.to_string());
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(key: &str, value: Value) -> SettingRow {
        SettingRow { key: key.into(), value }
    }

    #[test]
    fn defaults_fill_missing_keys_and_company_merges_deep() {
        let got = resolve_settings(&[
            row("gst_rate", json!(0.12)),
            row("company", json!({ "name": "LL", "gstin": "29ABC" })),
            row("unrelated_future_key", json!([1, 2])),
        ]);
        assert!(got.rejected.is_empty());
        let s = got.settings;
        assert_eq!(s.gst_rate, dec!(0.12));
        assert_eq!(
            (s.company.name.as_str(), s.company.gstin.as_str(), s.company.bank_ifsc.as_str()),
            ("LL", "29ABC", "")
        );
        assert_eq!(s.doc_prefixes.quotation, "LLQT-");
        assert_eq!(s.product_colors, ["Walnut", "Natural", "Black"]);
    }

    #[test]
    fn a_bad_value_keeps_its_default_and_is_reported() {
        let got = resolve_settings(&[row("hamper_statuses", json!("not a list")), row("gst_rate", json!("0.05"))]);
        assert_eq!(got.rejected, ["hamper_statuses"]);
        assert_eq!(got.settings.hamper_statuses, Settings::default().hamper_statuses);
        assert_eq!(got.settings.gst_rate, dec!(0.05));
    }

    #[test]
    fn merge_list_adds_only_new_values() {
        let existing = strings(&["Draft", "Approved"]);
        let found = [Some("approved"), Some(" Festive "), None, Some(""), Some("festive")];
        assert_eq!(merge_list(&existing, &found), ["Draft", "Approved", "Festive"]);
    }
}
