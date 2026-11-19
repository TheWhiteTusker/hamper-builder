//! `/hampers/H128/image`: the hamper's picture in the studio (port of
//! `hampers/[code]/image/*`). Saving stores the design and the PNG drawn
//! from it, which becomes the hamper's image.

use super::hamper_href;
use crate::data::use_catalog;
use crate::db::{self, PHOTOS, Q, me};
use crate::studio::{PickerProduct, SaveFn, Studio, StudioConfig};
use futures::FutureExt;
use lattice_core::canvas::{Canvas, parse_canvas};
use lattice_core::records::Hamper;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use serde_json::json;
use std::rc::Rc;

/// "…/object/public/product-images/hampers/x/y.png?v=1" → "hampers/x/y.png".
fn storage_path_of(url: &str) -> Option<String> {
    url.split(&format!("/{PHOTOS}/")).nth(1).map(|p| p.split('?').next().unwrap_or(p).to_string())
}

async fn save_hamper_canvas(id: String, code: String, canvas: Canvas, png: Option<Vec<u8>>) -> Result<(), String> {
    let png = png.filter(|p| !p.is_empty()).ok_or("The image could not be rendered.")?;
    if !canvas.valid() {
        return Err("Invalid design.".into());
    }
    let path = format!("hampers/{id}/cover-{}.png", js_sys::Date::now() as u64);
    let url = db::upload(PHOTOS, &path, &png, "image/png", true).await.map_err(|e| format!("Storage upload failed: {e}"))?;
    #[derive(Deserialize)]
    struct Before {
        image_url: Option<String>,
    }
    let before = Q::from("hampers").select("image_url").eq("id", &id).one::<Before>().await.ok().flatten();
    let updated = Q::from("hampers").eq("id", &id).update(&json!({ "canvas": canvas, "image_url": url }), Some("id")).await;
    let ok = updated.as_ref().is_ok_and(|v| v.as_array().is_some_and(|a| !a.is_empty()));
    if !ok {
        let _ = db::remove_files(&[path]).await;
        return Err(updated.err().map_or_else(|| "You don't have permission to edit this hamper.".into(), |e| e.to_string()));
    }
    // The previous PNG goes, if it was one of this hamper's own.
    if let Some(old) = before.and_then(|b| b.image_url).as_deref().and_then(storage_path_of).filter(|p| p.starts_with(&format!("hampers/{id}/"))) {
        let _ = db::remove_files(&[old]).await;
    }
    db::cache_drop(&["hampers", &format!("hamper:{code}"), "quote_options"]).await;
    Ok(())
}

#[derive(Clone)]
struct Loaded {
    hamper: Hamper,
    product_ids: Vec<String>,
}

#[component]
pub fn HamperImage() -> impl IntoView {
    if !me().role.can_manage() {
        return view! { <div class="card p-6 text-sm text-(--color-muted)">"Only a manager or admin can design hamper images."</div> }.into_any();
    }
    let params = use_params_map();
    let code = params.with_untracked(|p| p.get("code").map(|c| js_sys::decode_uri_component(&c).map(String::from).unwrap_or(c)).unwrap_or_default());
    let loaded = RwSignal::new(None::<Result<Loaded, String>>);
    let catalog = use_catalog();
    {
        let code = code.clone();
        spawn_local(async move {
            let res: Result<Loaded, String> = async {
                let hamper = Q::from("hampers").select("*").eq("code", &code).null("deleted_at").one::<Hamper>().await.map_err(|e| e.to_string())?.ok_or("No such hamper.")?;
                #[derive(Deserialize)]
                struct Item {
                    product_id: Option<String>,
                }
                let items: Vec<Item> = Q::from("hamper_items").select("product_id").eq("hamper_id", &hamper.id).rows().await.map_err(|e| e.to_string())?;
                let mut product_ids: Vec<String> = Vec::new();
                for p in items.into_iter().filter_map(|i| i.product_id) {
                    if !product_ids.contains(&p) {
                        product_ids.push(p);
                    }
                }
                Ok(Loaded { hamper, product_ids })
            }
            .await;
            loaded.set(Some(res));
        });
    }
    // Mounted once, when the hamper and the catalogue are both in.
    let ready = RwSignal::new_local(None::<StudioConfig>);
    Effect::new(move |_| {
        if ready.with_untracked(Option::is_some) {
            return;
        }
        let (Some(Ok(l)), Some(products)) = (loaded.get(), catalog.data.get()) else { return };
        let h = l.hamper;
        let (id, code) = (h.id.clone(), h.code.clone());
        let on_save: SaveFn = Rc::new(move |canvas, png| save_hamper_canvas(id.clone(), code.clone(), canvas, png).boxed_local());
        ready.set(Some(StudioConfig {
            title: h.name.clone(),
            subtitle: h.code.clone(),
            back_href: format!("{}/edit", hamper_href(&h.code)),
            download_name: h.code.clone(),
            initial: h.canvas.as_ref().map_or_else(lattice_core::canvas::empty_canvas, parse_canvas),
            products: products.into_iter().map(|p| PickerProduct { id: p.id, code: p.code, name: p.name, image_url: p.image_url }).collect(),
            hamper_product_ids: l.product_ids,
            on_save,
            save_image: true,
            saved_message: "Saved. The hamper image is updated.".into(),
            upload_folder: format!("hampers/{}", h.id),
            pages: Vec::new(),
            page_id: None,
        }));
    });
    view! {
        <Show
            when=move || ready.with(Option::is_some)
            fallback=move || match loaded.get() {
                Some(Err(e)) => view! { <div class="card p-6 text-sm text-red-800">{e}</div> }.into_any(),
                _ => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
            }
        >
            {move || ready.get_untracked().map(|config| view! { <Studio config=config /> })}
        </Show>
    }
    .into_any()
}
