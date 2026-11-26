//! GSTINs: the shape check, and reading the GST register's answer
//! (port of `lookupGstin` in `clients/actions.ts`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 2-digit state code, PAN (5 letters, 4 digits, 1 letter), entity number
/// (1-9 or A-Z), "Z", checksum: `^\d{2}[A-Z]{5}\d{4}[A-Z][1-9A-Z]Z[0-9A-Z]$`.
pub fn is_valid(gstin: &str) -> bool {
    let b = gstin.as_bytes();
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    let upper = |i: usize| b.get(i).is_some_and(u8::is_ascii_uppercase);
    b.len() == 15
        && (0..2).all(digit)
        && (2..7).all(upper)
        && (7..11).all(digit)
        && upper(11)
        && b.get(12).is_some_and(|c| matches!(c, b'1'..=b'9' | b'A'..=b'Z'))
        && b.get(13) == Some(&b'Z')
        && b.get(14).is_some_and(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GstDetails {
    pub name: String,
    pub billing_address: String,
}

/// gstinapi.in's answer, read leniently on purpose: the API adds fields
/// without notice. `ok` is whether the HTTP status was a success.
pub fn read_gstinapi(ok: bool, body: &Value) -> Result<GstDetails, String> {
    let data = body.get("data").filter(|d| d.is_object());
    if !ok || body.get("success").and_then(Value::as_bool) != Some(true) || data.is_none() {
        let fallback = "GST lookup failed. Enter the details by hand.";
        return Err(match body.get("error") {
            Some(Value::String(s)) => s.clone(),
            Some(o @ Value::Object(_)) => ["message", "error"]
                .iter()
                .find_map(|k| o.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()))
                .unwrap_or(fallback)
                .to_string(),
            _ => fallback.to_string(),
        });
    }
    let field = |k: &str| data.and_then(|d| d.get(k)).and_then(Value::as_str).filter(|s| !s.is_empty());
    Ok(GstDetails {
        name: field("trade_name").or_else(|| field("legal_name")).unwrap_or_default().to_string(),
        billing_address: [field("address"), field("pincode")].into_iter().flatten().collect::<Vec<_>>().join(" - "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shape() {
        assert!(is_valid("29AAECB4326R1Z5"));
        assert!(!is_valid("29aaecb4326r1z5"), "lower case is refused; callers upper-case first");
        assert!(!is_valid("29AAECB4326R0Z5"), "entity number cannot be 0");
        assert!(!is_valid("29AAECB4326R1Y5"));
        assert!(!is_valid("29AAECB4326R1Z"));
    }

    #[test]
    fn register_answers() {
        let ok = json!({"success": true, "data": {"trade_name": "", "legal_name": "Acme Pvt Ltd", "address": "1 MG Road", "pincode": "560001"}});
        assert_eq!(
            read_gstinapi(true, &ok),
            Ok(GstDetails { name: "Acme Pvt Ltd".into(), billing_address: "1 MG Road - 560001".into() })
        );
        assert_eq!(read_gstinapi(false, &json!({"error": "Invalid key"})), Err("Invalid key".into()));
        assert_eq!(
            read_gstinapi(true, &json!({"success": false, "error": {"message": "Not found"}})),
            Err("Not found".into())
        );
        assert_eq!(read_gstinapi(true, &Value::Null), Err("GST lookup failed. Enter the details by hand.".into()));
    }
}
