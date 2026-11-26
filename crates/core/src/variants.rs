//! Colour siblings of a product (port of `product-variants.ts`).

use crate::product_code::parse_product_code;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductVariant {
    pub id: String,
    pub code: String,
    pub colors: Option<Vec<String>>,
}

/// Product id for each colour in a family of sibling variants.
pub fn variant_ids_by_color(products: &[ProductVariant]) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::new();
    for p in products {
        let colors: Vec<&String> = p.colors.iter().flatten().filter(|c| !c.is_empty()).collect();
        if colors.is_empty() {
            // Legacy variants may have no colours array, but the code carries the finish.
            ids.insert(parse_product_code(&p.code).color_name, p.id.clone());
        } else {
            for c in colors {
                ids.insert(c.clone(), p.id.clone());
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(id: &str, code: &str, colors: Option<&[&str]>) -> ProductVariant {
        ProductVariant {
            id: id.into(),
            code: code.into(),
            colors: colors.map(|c| c.iter().map(|s| (*s).to_string()).collect()),
        }
    }

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn maps_each_finish_to_its_sibling() {
        let got = variant_ids_by_color(&[
            v("walnut-id", "LC/0001/WL", Some(&["Walnut"])),
            v("natural-id", "LC/0001/NT", Some(&["Natural"])),
            v("black-id", "LC/0001/BL", Some(&["Black"])),
        ]);
        assert_eq!(got, map(&[("Walnut", "walnut-id"), ("Natural", "natural-id"), ("Black", "black-id")]));
    }

    #[test]
    fn legacy_variants_use_the_code() {
        let got = variant_ids_by_color(&[v("walnut-id", "LC/0001/WL", None), v("natural-id", "LC/0001/NT", Some(&[]))]);
        assert_eq!(got, map(&[("Walnut", "walnut-id"), ("Natural", "natural-id")]));
    }
}
