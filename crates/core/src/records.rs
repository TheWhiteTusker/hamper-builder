//! Row shapes for the tables and views in `supabase/migrations` (port of
//! `types.ts` and `types-docs.ts`). Numeric columns read leniently, so a
//! `numeric` that arrives as a string or null never fails a whole page.

use crate::lenient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Manager,
    #[default]
    Sales,
}

impl UserRole {
    /// Manager or admin: builds hampers, edits any quote, changes status.
    pub fn can_manage(self) -> bool {
        matches!(self, Self::Admin | Self::Manager)
    }

    pub fn is_admin(self) -> bool {
        self == Self::Admin
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    #[default]
    Quotation,
    ProformaInvoice,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub full_name: Option<String>,
    pub role: UserRole,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub counts_as_item: bool,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Product {
    pub id: String,
    pub code: String,
    pub category_id: Option<String>,
    pub name: String,
    pub source: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub cost_price: Decimal,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub markup_pct: Option<Decimal>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub target_margin: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub default_sp: Decimal,
    #[serde(deserialize_with = "lenient::null_default")]
    pub colors: Vec<String>,
    pub image_url: Option<String>,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

/// A product's category as embedded by `select=*,categories(name,counts_as_item)`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CategoryRef {
    pub name: String,
    pub counts_as_item: bool,
}

/// A product joined to its category, the shape lists and the builders need.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductWithCategory {
    #[serde(flatten)]
    pub product: Product,
    pub categories: Option<CategoryRef>,
}

/// One row of `product_cost_sheets` (its lines are loaded separately).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductCostSheet {
    pub id: String,
    pub product_id: Option<String>,
    pub product_code: Option<String>,
    #[serde(deserialize_with = "lenient::null_default")]
    pub product_name: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub material_total: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub hardware_total: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub finishing_total: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub machine_total: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_cost: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub markup_pct: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub calculated_sp: Decimal,
    /// Overhead % per stage code, already included in the stage totals.
    #[serde(deserialize_with = "lenient::decimal_map")]
    pub stage_overheads: crate::costing::StageOverheads,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hamper {
    pub id: String,
    pub code: String,
    pub name: String,
    pub collection: Option<String>,
    pub status: String,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub target_sp: Option<Decimal>,
    pub notes: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub discount_pct: Decimal,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub final_catalogue_sp: Option<Decimal>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    /// Designer layout; parsed by the studio crate.
    pub canvas: Option<serde_json::Value>,
    pub image_url: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HamperItem {
    pub id: String,
    pub hamper_id: String,
    pub line_no: i32,
    pub product_id: Option<String>,
    pub product_code: Option<String>,
    pub product_name: String,
    pub category_name: Option<String>,
    pub source: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub qty: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub unit_cp: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub unit_sp: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub target_margin: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_cp: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_sp: Decimal,
}

/// The `hamper_summary` view: what the Hamper Summary sheet used to hold.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HamperSummary {
    pub id: String,
    pub code: String,
    pub name: String,
    pub collection: Option<String>,
    pub status: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub number_of_items: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_cp: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub base_sp: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub discount_pct: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub sp_after_discount: Decimal,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub target_sp: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub variance: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub final_catalogue_sp: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub gross_profit: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub final_margin: Option<Decimal>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub line_count: i64,
    pub image_url: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Quote {
    pub id: String,
    pub doc_no: String,
    pub doc_type: DocType,
    pub doc_date: Option<String>,
    pub client_name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub billing_address: Option<String>,
    pub gstin: Option<String>,
    pub occasion: Option<String>,
    pub quote_structure: String,
    pub validity: Option<String>,
    pub status: String,
    #[serde(deserialize_with = "lenient::decimal")]
    pub order_discount: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub adj1: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub adj2: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub gst_rate: Decimal,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub follow_up_date: Option<String>,
    pub linked_doc_no: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuoteItem {
    pub id: String,
    pub quote_id: String,
    pub line_no: i32,
    pub option_label: Option<String>,
    pub hamper_id: Option<String>,
    pub hamper_code: Option<String>,
    pub hamper_name: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub qty: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub catalogue_price: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub discount_pct: Decimal,
    pub detail_mode: Option<String>,
    pub packaging_treatment: Option<String>,
    #[serde(deserialize_with = "lenient::decimal")]
    pub final_rate: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub amount: Decimal,
}

/// The `quote_summary` view.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuoteSummary {
    #[serde(flatten)]
    pub quote: Quote,
    #[serde(deserialize_with = "lenient::decimal")]
    pub subtotal: Decimal,
    #[serde(deserialize_with = "lenient::decimal")]
    pub total_qty: Decimal,
    pub line_count: i64,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub taxable_value: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub gst_amount: Option<Decimal>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    pub grand_total: Option<Decimal>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub gstin: Option<String>,
    pub billing_address: Option<String>,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn view_rows_read_with_nulls_and_extra_columns() -> Result<(), serde_json::Error> {
        let row: HamperSummary = serde_json::from_str(
            r#"{"id":"1","code":"H001","name":"Box","status":"Draft","total_cp":950,"base_sp":"1550",
                "target_sp":null,"final_margin":0.3667,"line_count":3,"unknown_future_column":true}"#,
        )?;
        assert_eq!((row.total_cp, row.base_sp, row.target_sp), (dec!(950), dec!(1550), None));
        assert_eq!(row.final_margin, Some(dec!(0.3667)));

        let q: QuoteSummary = serde_json::from_str(
            r#"{"id":"q","doc_no":"LLQT-001","doc_type":"proforma_invoice","client_name":"Acme","grand_total":null,"subtotal":10}"#,
        )?;
        assert_eq!((q.quote.doc_type, q.subtotal, q.grand_total), (DocType::ProformaInvoice, dec!(10), None));
        Ok(())
    }

    #[test]
    fn roles() {
        assert!(UserRole::Manager.can_manage() && !UserRole::Sales.can_manage());
        assert!(UserRole::Admin.is_admin() && !UserRole::Manager.is_admin());
    }
}
