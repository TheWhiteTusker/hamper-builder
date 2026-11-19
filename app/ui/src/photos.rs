//! Product photos: resized with the webview's own canvas encoders (as the
//! Next app did in the browser), uploaded through the native side, and the
//! gallery actions (ports of `upload-product-image.ts`, `image-utils.ts`,
//! `image-actions.ts`, `image-edit-actions.ts`).

use crate::db::{self, PHOTOS, Q};
use js_sys::{Promise, Uint8Array};
use lattice_core::api::ApiError;
use lattice_core::colors::color_tag;
use lattice_core::images::{ProductImage, thumb_path, view_path};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, HtmlCanvasElement, HtmlImageElement};

fn js_err(what: &str) -> impl Fn(JsValue) -> ApiError + '_ {
    move |e| {
        ApiError::Invalid(format!("{what}: {}", e.as_string().unwrap_or_else(|| "the image could not be read".into())))
    }
}

pub async fn blob_bytes(blob: &Blob) -> Result<Vec<u8>, ApiError> {
    let buf = JsFuture::from(blob.array_buffer()).await.map_err(js_err("Could not read the file"))?;
    Ok(Uint8Array::new(&buf).to_vec())
}

fn canvas(w: u32, h: u32) -> Result<(HtmlCanvasElement, web_sys::CanvasRenderingContext2d), ApiError> {
    let doc = leptos::prelude::document();
    let c: HtmlCanvasElement = doc
        .create_element("canvas")
        .map_err(js_err("No canvas"))?
        .dyn_into()
        .map_err(|_| ApiError::Server("No canvas".into()))?;
    c.set_width(w);
    c.set_height(h);
    let ctx = c
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|x| x.dyn_into().ok())
        .ok_or_else(|| ApiError::Server("No 2D canvas".into()))?;
    Ok((c, ctx))
}

/// `canvas.toBlob(type, quality)` as a future; None when the browser can't encode.
pub async fn to_blob(c: &HtmlCanvasElement, mime: &str, quality: Option<f64>) -> Result<Option<Blob>, ApiError> {
    let (c, mime) = (c.clone(), mime.to_string());
    let promise = Promise::new(&mut |resolve, _reject| {
        let cb = Closure::once_into_js(move |b: JsValue| {
            let _ = resolve.call1(&JsValue::NULL, &b);
        });
        let q = quality.map_or(JsValue::UNDEFINED, JsValue::from_f64);
        let _ = c.to_blob_with_type_and_encoder_options(cb.unchecked_ref(), &mime, &q);
    });
    let v = JsFuture::from(promise).await.map_err(js_err("Could not encode the image"))?;
    Ok(v.dyn_into::<Blob>().ok())
}

/// An `<img>` of the file, decoded.
async fn load_image(blob: &Blob) -> Result<(HtmlImageElement, String), ApiError> {
    let url = web_sys::Url::create_object_url_with_blob(blob).map_err(js_err("Could not open the file"))?;
    let img = HtmlImageElement::new().map_err(js_err("No image support"))?;
    img.set_src(&url);
    JsFuture::from(img.decode()).await.map_err(|_| {
        let _ = web_sys::Url::revoke_object_url(&url);
        ApiError::Invalid("That file is not an image this computer can read.".into())
    })?;
    Ok((img, url))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scaled(w: u32, h: u32, max: f64) -> (u32, u32) {
    let scale = (max / f64::from(w.max(h))).min(1.0);
    (lattice_core::js::round(f64::from(w) * scale) as u32, lattice_core::js::round(f64::from(h) * scale) as u32)
}

/// A WebP copy no larger than `max` px (PNG if this webview can't encode WebP).
pub async fn to_webp(file: &Blob, max: f64, quality: f64) -> Result<(Vec<u8>, String), ApiError> {
    let (img, url) = load_image(file).await?;
    let (w, h) = scaled(img.natural_width(), img.natural_height(), max);
    let (c, ctx) = canvas(w, h)?;
    let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, f64::from(w), f64::from(h));
    let _ = web_sys::Url::revoke_object_url(&url);
    let blob = to_blob(&c, "image/webp", Some(quality))
        .await?
        .ok_or_else(|| ApiError::Server("Could not process that image.".into()))?;
    let mime = blob.type_();
    Ok((blob_bytes(&blob).await?, mime))
}

/// The original to store: large camera photos scaled to 2000px and compressed;
/// anything under 1 MB (or that would not get smaller) kept as it is.
pub async fn optimize(file: &File) -> Result<(Vec<u8>, String, String), ApiError> {
    let name = file.name();
    let mime = file.type_();
    let ext = name
        .rsplit_once('.')
        .map_or_else(|| mime.split('/').nth(1).unwrap_or_default().to_string(), |(_, e)| e.to_lowercase());
    let keep = || async { Ok::<_, ApiError>((blob_bytes(file).await?, mime.clone(), ext.clone())) };
    if !mime.starts_with("image/") || mime == "image/svg+xml" || file.size() <= 1024.0 * 1024.0 {
        return keep().await;
    }
    let Ok((img, url)) = load_image(file).await else { return keep().await };
    let (w0, h0) = (img.natural_width(), img.natural_height());
    let scale = (2000.0 / f64::from(w0.max(h0))).min(1.0);
    if (scale - 1.0).abs() < f64::EPSILON && file.size() < 2.0 * 1024.0 * 1024.0 {
        let _ = web_sys::Url::revoke_object_url(&url);
        return keep().await;
    }
    let (w, h) = scaled(w0, h0, 2000.0);
    let (c, ctx) = canvas(w, h)?;
    let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, f64::from(w), f64::from(h));
    let _ = web_sys::Url::revoke_object_url(&url);
    let png = mime == "image/png" || name.to_lowercase().ends_with(".png");
    let (out_mime, out_ext, q) = if png { ("image/png", "png", None) } else { ("image/jpeg", "jpg", Some(0.85)) };
    match to_blob(&c, out_mime, q).await? {
        Some(b) if b.size() < file.size() => Ok((blob_bytes(&b).await?, out_mime.into(), out_ext.into())),
        _ => keep().await,
    }
}

const IMAGE_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// A fresh object name under `folder`, like `products/<id>/asset-1700000000000-ab12.png`.
pub fn asset_path(folder: &str, ext: &str) -> String {
    let rand: String = (0..4)
        .map(|_| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = (js_sys::Math::random() * 36.0) as u32;
            char::from_digit(i.min(35), 36).unwrap_or('0')
        })
        .collect();
    format!("{folder}/asset-{}-{rand}.{ext}", js_sys::Date::now() as u64)
}

/// Uploads a photo (the original plus 2000px and 480px WebP copies), then
/// records it on the product. The first photo of a product becomes its cover.
pub async fn upload_product_image(
    file: &File,
    product_id: &str,
    color: Option<&str>,
    is_primary: bool,
) -> Result<ProductImage, ApiError> {
    if !file.type_().starts_with("image/") {
        return Err(ApiError::Invalid("That file is not an image.".into()));
    }
    let (bytes, mime, ext) = optimize(file).await?;
    if !IMAGE_EXTS.contains(&ext.as_str()) {
        return Err(ApiError::Invalid("That file is not a supported image (PNG, JPG, WebP or GIF).".into()));
    }
    // A format this webview can't decode gets no copies: viewers fall back to the original.
    let view = to_webp(file, 2000.0, 0.85).await.ok();
    let thumb = to_webp(file, 480.0, 0.8).await.ok();
    let path = asset_path(&format!("products/{product_id}"), &ext);
    let url = db::upload(PHOTOS, &path, &bytes, &mime, false).await?;
    for (copy_path, copy) in [(thumb_path(&path), thumb), (view_path(&path), view)] {
        if let Some((b, m)) = copy {
            let _ = db::upload(PHOTOS, &copy_path, &b, &m, false).await;
        }
    }
    record_image(product_id, &url, Some(&path), color, is_primary).await
}

/// Step 2 of an upload (`saveUploadedProductImage`), also used for photos added by URL.
pub async fn record_image(
    product_id: &str,
    url: &str,
    storage_path: Option<&str>,
    color: Option<&str>,
    is_primary: bool,
) -> Result<ProductImage, ApiError> {
    let products = Q::from("products").eq("id", product_id);
    if is_primary {
        Q::from("product_images").eq("product_id", product_id).update(&json!({ "is_primary": false }), None).await?;
    } else if storage_path.is_some() {
        let count =
            Q::from("product_images").select("id").eq("product_id", product_id).null("deleted_at").count().await?;
        if count == 0 {
            products.update(&json!({ "image_url": url }), None).await?;
        }
    }
    let tag = color_tag(color);
    let row = json!({
        "product_id": product_id,
        "url": url,
        "storage_path": storage_path,
        "color": tag.color,
        "color_code": tag.color_code,
        "is_primary": is_primary,
        "caption": null,
    });
    let inserted: Vec<ProductImage> = db::insert_returning("product_images", &row).await?;
    let image = inserted.into_iter().next().ok_or_else(|| ApiError::Server("Database save failed.".into()))?;
    if is_primary {
        products.update(&json!({ "image_url": url }), None).await?;
    }
    Ok(image)
}

fn first(v: serde_json::Value) -> Result<ProductImage, ApiError> {
    serde_json::from_value::<Vec<ProductImage>>(v)
        .ok()
        .and_then(|v| v.into_iter().next())
        .ok_or_else(|| ApiError::NotFound("Image not found.".into()))
}

/// Makes one photo the product's cover.
pub async fn set_primary(image_id: &str, product_id: &str) -> Result<ProductImage, ApiError> {
    Q::from("product_images").eq("product_id", product_id).update(&json!({ "is_primary": false }), None).await?;
    let updated =
        first(Q::from("product_images").eq("id", image_id).update(&json!({ "is_primary": true }), Some("*")).await?)?;
    Q::from("products").eq("id", product_id).update(&json!({ "image_url": updated.url }), None).await?;
    Ok(updated)
}

/// Re-tags a photo's colour ("General" or None for all colours).
pub async fn change_color(image_id: &str, raw: Option<&str>) -> Result<ProductImage, ApiError> {
    let tag = color_tag(raw);
    first(
        Q::from("product_images")
            .eq("id", image_id)
            .update(&json!({ "color": tag.color, "color_code": tag.color_code }), Some("*"))
            .await?,
    )
}

/// Moves a photo to the Bin; if it was the cover, the next one takes over.
/// Returns the photo that became the cover, if any.
pub async fn delete_image(image_id: &str, product_id: &str) -> Result<Option<ProductImage>, ApiError> {
    let img = Q::from("product_images")
        .select("*")
        .eq("id", image_id)
        .one::<ProductImage>()
        .await?
        .ok_or_else(|| ApiError::NotFound("Image not found.".into()))?;
    let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
    Q::from("product_images")
        .eq("id", image_id)
        .update(&json!({ "deleted_at": now, "is_primary": false }), None)
        .await?;
    if !img.is_primary {
        return Ok(None);
    }
    let next = Q::from("product_images")
        .select("*")
        .eq("product_id", product_id)
        .null("deleted_at")
        .order("sort_order")
        .order("created_at")
        .one::<ProductImage>()
        .await?;
    let products = Q::from("products").eq("id", product_id);
    match next {
        Some(n) => {
            Q::from("product_images").eq("id", &n.id).update(&json!({ "is_primary": true }), None).await?;
            products.update(&json!({ "image_url": n.url }), None).await?;
            Ok(Some(ProductImage { is_primary: true, ..n }))
        }
        None => {
            products.update(&json!({ "image_url": null }), None).await?;
            Ok(None)
        }
    }
}
