//! Loaders shared by several screens (ports of `lib/settings.ts` and
//! `cost-calculator/load.ts`), each cached on the device under a key.

use crate::db::{Live, Q, live, live_with};
use futures::join;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::cost_tree::{CostCategory, CostRows, CostStage, CostSubcategory, CostVariety};
use lattice_core::costing::ProductCostLine;
use lattice_core::images::ProductImage;
use lattice_core::lenient;
use lattice_core::product_code::parse_product_code;
use lattice_core::records::{Category, Product, ProductCostSheet};
use lattice_core::settings::{SettingRow, Settings, resolve_settings};
use lattice_core::variants::{ProductVariant, variant_ids_by_color};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub async fn load_settings() -> Result<Settings, ApiError> {
    let rows: Vec<SettingRow> = Q::from("app_settings").select("key,value").rows().await?;
    Ok(resolve_settings(&rows).settings)
}

/// Everything in `app_settings`, merged over the defaults.
pub fn use_settings() -> Live<Settings> {
    live_with("settings", load_settings)
}

pub fn categories_query() -> Q {
    Q::from("categories").select("*").order("sort_order").order("name")
}

pub fn use_categories() -> Live<Vec<Category>> {
    live("categories", categories_query())
}

fn active(table: &str) -> Q {
    Q::from(table).select("*").eq("is_active", "true").order("sort_order")
}

/// The costing hierarchy's four tables, active rows only.
pub async fn load_cost_rows() -> Result<CostRows, ApiError> {
    let (st, ca, su, va) =
        (active("cost_stages"), active("cost_categories"), active("cost_subcategories"), active("cost_varieties"));
    let (stages, categories, subcategories, varieties) = join!(
        st.rows::<CostStage>(),
        ca.rows::<CostCategory>(),
        su.rows::<CostSubcategory>(),
        va.rows::<CostVariety>(),
    );
    Ok(CostRows { stages: stages?, categories: categories?, subcategories: subcategories?, varieties: varieties? })
}

pub fn use_cost_rows() -> Live<CostRows> {
    live_with("cost_rows", load_cost_rows)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SheetWithLines {
    #[serde(flatten)]
    pub sheet: ProductCostSheet,
    pub lines: Vec<ProductCostLine>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sibling {
    pub id: String,
    pub code: String,
    pub color: String,
    pub sp: Decimal,
}

/// A product with its cost sheet, photos and colour family.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProductCosting {
    /// Its `colors` include every sibling's, as the calculator shows them.
    pub product: Product,
    /// This product's own colour, before the siblings' are merged in; "" when never set.
    pub own_color: String,
    pub sheet: Option<SheetWithLines>,
    pub images: Vec<ProductImage>,
    pub variant_ids: BTreeMap<String, String>,
    /// The other colours, offered the new price when this one is saved.
    pub siblings: Vec<Sibling>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct FamilyRow {
    id: String,
    code: String,
    colors: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient::opt_decimal")]
    default_sp: Option<Decimal>,
}

/// Port of `loadProductCosting`: None when the code is unknown (or binned).
pub async fn load_product_costing(code: &str) -> Result<Option<ProductCosting>, ApiError> {
    let Some(product) = Q::from("products").select("*").eq("code", code).null("deleted_at").one::<Product>().await?
    else {
        return Ok(None);
    };

    let parsed = parse_product_code(&product.code);
    let mut colors = product.colors.clone();
    let mut family = vec![FamilyRow {
        id: product.id.clone(),
        code: product.code.clone(),
        colors: Some(product.colors.clone()),
        default_sp: Some(product.default_sp),
    }];
    if !parsed.category_code.is_empty() && !parsed.serial.is_empty() {
        let pattern = format!("{}/{}/*", parsed.category_code, parsed.serial);
        let siblings: Vec<FamilyRow> = Q::from("products")
            .select("id,code,colors,default_sp")
            .ilike("code", &pattern)
            .null("deleted_at")
            .rows()
            .await?;
        for s in siblings.into_iter().filter(|s| s.id != product.id) {
            for c in s.colors.iter().flatten() {
                if !colors.contains(c) {
                    colors.push(c.clone());
                }
            }
            family.push(s);
        }
    }

    let sheet_q = Q::from("product_cost_sheets").select("*").eq("product_id", &product.id);
    let images_q = Q::from("product_images")
        .select("*")
        // Each colour is its own product: only this one's photos, never a sibling's.
        .eq("product_id", &product.id)
        .null("deleted_at")
        .order("sort_order")
        .order("created_at");
    let (sheet, images) = join!(sheet_q.one::<ProductCostSheet>(), images_q.rows::<ProductImage>());
    let sheet = match sheet? {
        Some(sheet) => {
            let lines =
                Q::from("product_cost_lines").select("*").eq("sheet_id", &sheet.id).order("sort_order").rows().await?;
            Some(SheetWithLines { sheet, lines })
        }
        None => None,
    };

    let variants: Vec<ProductVariant> = family
        .iter()
        .map(|f| ProductVariant { id: f.id.clone(), code: f.code.clone(), colors: f.colors.clone() })
        .collect();
    let own_color = product.colors.first().cloned().unwrap_or_default();
    let siblings = family
        .iter()
        .skip(1)
        .map(|s| Sibling {
            id: s.id.clone(),
            code: s.code.clone(),
            color: s.colors.as_ref().and_then(|c| c.first().cloned()).unwrap_or_else(|| s.code.clone()),
            sp: s.default_sp.unwrap_or_default(),
        })
        .collect();
    let product = Product { colors: if colors.is_empty() { product.colors.clone() } else { colors }, ..product };
    Ok(Some(ProductCosting {
        product,
        own_color,
        sheet,
        images: images?,
        variant_ids: variant_ids_by_color(&variants),
        siblings,
    }))
}

/// The slice of Product Master the builders need, loaded up front so every
/// picker filters on the device (port of `hampers/data.ts`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogProduct {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub counts_as_item: bool,
    pub source: Option<String>,
    pub cost_price: Decimal,
    pub markup_pct: Decimal,
    pub target_margin: Decimal,
    pub default_sp: Decimal,
    pub colors: Vec<String>,
    pub image_url: Option<String>,
}

pub async fn load_catalog() -> Result<Vec<CatalogProduct>, ApiError> {
    let rows: Vec<lattice_core::records::ProductWithCategory> = Q::from("products")
        .select("id,code,name,category_id,source,cost_price,markup_pct,target_margin,default_sp,colors,image_url,categories(name,counts_as_item)")
        .eq("is_active", "true")
        .null("deleted_at")
        .order("name")
        .rows()
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let p = r.product;
            CatalogProduct {
                counts_as_item: r.categories.as_ref().is_none_or(|c| c.counts_as_item),
                category_name: r.categories.map(|c| c.name),
                id: p.id,
                code: p.code,
                name: p.name,
                category_id: p.category_id,
                source: p.source,
                cost_price: p.cost_price,
                markup_pct: p.markup_pct.unwrap_or_default(),
                target_margin: p.target_margin,
                default_sp: p.default_sp,
                colors: p.colors,
                image_url: p.image_url,
            }
        })
        .collect())
}

pub fn use_catalog() -> Live<Vec<CatalogProduct>> {
    live_with("catalog", load_catalog)
}
