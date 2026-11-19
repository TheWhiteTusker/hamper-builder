//! Product photos by colour, and their thumbnail/view copies (port of `product-images.ts`).

use crate::colors::{code_to_color, color_code, color_to_code};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductImage {
    pub id: String,
    pub product_id: String,
    pub url: String,
    pub storage_path: Option<String>,
    /// "Walnut", "Natural", "Black", any Settings colour, or None for all colours.
    pub color: Option<String>,
    /// "WL", "NT", "BL", …
    pub color_code: Option<String>,
    pub is_primary: bool,
    pub sort_order: i32,
    pub caption: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|s| !s.is_empty())
}

/// Images for one colour (name or code). With `include_general`, untagged
/// images match too, and every image is returned when none match.
pub fn images_for_color<'a>(
    images: &'a [ProductImage],
    color_or_code: Option<&str>,
    include_general: bool,
) -> Vec<&'a ProductImage> {
    let Some(wanted) = color_or_code.filter(|c| !c.is_empty()) else { return images.iter().collect() };
    let code = color_to_code(&wanted.trim().to_lowercase()).map_or_else(|| color_code(wanted), str::to_string);
    let name = code_to_color(&code).unwrap_or(wanted).to_lowercase();

    let filtered: Vec<&ProductImage> = images
        .iter()
        .filter(|img| {
            let (img_color, img_code) = (non_empty(&img.color), non_empty(&img.color_code));
            if img_color.is_none() && img_code.is_none() {
                return include_general;
            }
            let this_code = img_code.map(str::to_uppercase).or_else(|| img_color.map(color_code)).unwrap_or_default();
            let this_name = img_color.or_else(|| img_code.and_then(code_to_color));
            this_code == code || this_name.is_some_and(|n| n.to_lowercase() == name)
        })
        .collect();

    if !filtered.is_empty() {
        filtered
    } else if include_general {
        images.iter().collect()
    } else {
        Vec::new()
    }
}

/// The best photo for a product: the primary of the preferred colour, any of
/// that colour, the overall primary, then the first.
pub fn primary_image<'a>(images: &'a [ProductImage], preferred: Option<&str>) -> Option<&'a ProductImage> {
    if let Some(color) = preferred.filter(|c| !c.is_empty()) {
        let of_color = images_for_color(images, Some(color), false);
        if let Some(img) = of_color.iter().find(|i| i.is_primary).or(of_color.first()) {
            return Some(img);
        }
    }
    images.iter().find(|i| i.is_primary).or(images.first())
}

#[derive(Debug, Default, PartialEq)]
pub struct ColorGroups<'a> {
    pub walnut: Vec<&'a ProductImage>,
    pub natural: Vec<&'a ProductImage>,
    pub black: Vec<&'a ProductImage>,
    pub general: Vec<&'a ProductImage>,
}

/// Images in the three standard colour buckets plus General.
pub fn group_images_by_color(images: &[ProductImage]) -> ColorGroups<'_> {
    let mut g = ColorGroups::default();
    for img in images {
        let raw = img.color.as_deref().unwrap_or_default().trim().to_lowercase();
        let code =
            non_empty(&img.color_code).map(str::to_uppercase).or_else(|| color_to_code(&raw).map(str::to_string));
        match (code.as_deref(), raw.as_str()) {
            (Some("WL"), _) | (_, "walnut") => g.walnut.push(img),
            (Some("NT"), _) | (_, "natural") => g.natural.push(img),
            (Some("BL"), _) | (_, "black") => g.black.push(img),
            _ => g.general.push(img),
        }
    }
    g
}

const PRODUCT_PHOTOS: &str = "/product-images/products/";

/// `products/x/asset-1.png` → `products/x/asset-1.<kind>.webp`.
fn copy_path(path: &str, kind: &str) -> String {
    let stem = match path.rfind('.') {
        Some(dot) if dot + 1 < path.len() && !path[dot + 1..].contains('/') => &path[..dot],
        _ => path,
    };
    format!("{stem}.{kind}.webp")
}

/// Each product photo is stored three ways: the original (placed by the
/// studio and presentations), a 2000px `.view.webp` and a 480px `.thumb.webp`.
pub fn thumb_path(path: &str) -> String {
    copy_path(path, "thumb")
}

pub fn view_path(path: &str) -> String {
    copy_path(path, "view")
}

/// The WebP copy of one of our product photos; any other URL is unchanged.
pub fn thumb_url(url: &str) -> String {
    if url.contains(PRODUCT_PHOTOS) { thumb_path(url) } else { url.to_string() }
}

pub fn view_url(url: &str) -> String {
    if url.contains(PRODUCT_PHOTOS) { view_path(url) } else { url.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(id: &str, color: Option<&str>, code: Option<&str>, primary: bool) -> ProductImage {
        ProductImage {
            id: id.into(),
            color: color.map(Into::into),
            color_code: code.map(Into::into),
            is_primary: primary,
            ..Default::default()
        }
    }

    fn mock() -> Vec<ProductImage> {
        vec![
            img("img-1", Some("Walnut"), Some("WL"), true),
            img("img-2", Some("Walnut"), Some("WL"), false),
            img("img-3", Some("Natural"), Some("NT"), true),
            img("img-4", Some("Black"), Some("BL"), true),
            img("img-5", None, None, false),
        ]
    }

    fn ids(v: &[&ProductImage]) -> Vec<String> {
        v.iter().map(|i| i.id.clone()).collect()
    }

    #[test]
    fn filters_by_color_name_and_code() {
        let m = mock();
        assert_eq!(ids(&images_for_color(&m, Some("Walnut"), false)), ["img-1", "img-2"]);
        assert_eq!(ids(&images_for_color(&m, Some("NT"), false)), ["img-3"]);
        assert_eq!(ids(&images_for_color(&m, Some("Black"), false)), ["img-4"]);
        // No Teak photos: the untagged one stands in; without it, every photo does.
        assert_eq!(ids(&images_for_color(&m, Some("Teak"), true)), ["img-5"]);
        assert_eq!(images_for_color(&m[..4], Some("Teak"), true).len(), 4);
        assert!(images_for_color(&m, Some("Teak"), false).is_empty());
    }

    #[test]
    fn primary_for_a_color() {
        let m = mock();
        assert_eq!(primary_image(&m, Some("Natural")).map(|i| i.id.as_str()), Some("img-3"));
        assert_eq!(primary_image(&m, Some("WL")).map(|i| i.id.as_str()), Some("img-1"));
        assert_eq!(primary_image(&m, Some("BL")).map(|i| i.id.as_str()), Some("img-4"));
        assert_eq!(primary_image(&[], Some("BL")), None);
    }

    #[test]
    fn groups_into_color_buckets() {
        let m = mock();
        let g = group_images_by_color(&m);
        assert_eq!((g.walnut.len(), g.natural.len(), g.black.len(), g.general.len()), (2, 1, 1, 1));
    }

    #[test]
    fn thumb_and_view_copies() {
        let base = "https://x.supabase.co/storage/v1/object/public/product-images";
        assert_eq!(
            thumb_url(&format!("{base}/products/p1/asset-1-ab.png")),
            format!("{base}/products/p1/asset-1-ab.thumb.webp")
        );
        assert_eq!(thumb_path("products/p1/asset-1-ab.webp"), "products/p1/asset-1-ab.thumb.webp");
        assert_eq!(view_path("products/p1/asset-1-ab.png"), "products/p1/asset-1-ab.view.webp");
        assert_eq!(
            view_url(&format!("{base}/products/p1/asset-1-ab.png")),
            format!("{base}/products/p1/asset-1-ab.view.webp")
        );
        assert_eq!(view_url(&format!("{base}/hampers/h1/a.png")), format!("{base}/hampers/h1/a.png"));
        assert_eq!(thumb_url("https://example.com/walnut.jpg"), "https://example.com/walnut.jpg");
        assert_eq!(thumb_url("blob:http://localhost/123"), "blob:http://localhost/123");
        assert_eq!(thumb_path("a/b"), "a/b.thumb.webp");
    }
}
