//! `/presentations/<id>/slides/<slide>`: one slide in the studio, with the
//! deck's other slides in a strip underneath (port of `slides/[slideId]` and
//! `slide-actions.ts`).

use super::deck_href;
use crate::data::use_catalog;
use crate::db::{self, Q};
use crate::studio::{PageLink, PickerProduct, SaveFn, Studio, StudioConfig};
use futures::FutureExt;
use lattice_core::canvas::{Canvas, parse_canvas};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::Deserialize;
use serde_json::{Value, json};
use std::rc::Rc;

#[derive(Clone, Deserialize)]
struct Row {
    id: String,
    canvas: Value,
    hamper_id: Option<String>,
    product_id: Option<String>,
}

#[derive(Clone)]
struct Loaded {
    title: String,
    slides: Vec<Row>,
}

async fn save_slide(deck: String, slide: String, canvas: Canvas) -> Result<(), String> {
    if !canvas.valid() {
        return Err("Invalid design.".into());
    }
    let rows = Q::from("presentation_slides")
        .eq("id", &slide)
        .eq("presentation_id", &deck)
        .update(&json!({ "canvas": canvas }), Some("id"))
        .await
        .map_err(|e| e.to_string())?;
    if rows.as_array().is_none_or(Vec::is_empty) {
        return Err("This slide no longer exists.".into());
    }
    let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
    let _ = Q::from("presentations").eq("id", &deck).update(&json!({ "updated_at": now }), None).await;
    db::cache_drop(&["presentations", &format!("deck:{deck}")]).await;
    Ok(())
}

#[component]
pub fn SlideEditor() -> impl IntoView {
    let params = use_params_map();
    // A new slide id (from the strip) starts the editor again.
    let key =
        Memo::new(move |_| params.with(|p| (p.get("id").unwrap_or_default(), p.get("slide").unwrap_or_default())));
    view! { {move || { let (deck, slide) = key.get(); view! { <Editor deck=deck slide=slide /> } }} }
}

#[component]
fn Editor(deck: String, slide: String) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let catalog = use_catalog();
    let loaded = RwSignal::new(None::<Result<Option<Loaded>, String>>);
    {
        let deck = deck.clone();
        spawn_local(async move {
            let res = async {
                #[derive(Deserialize)]
                struct Head {
                    title: String,
                }
                let Some(head) = Q::from("presentations")
                    .select("title")
                    .eq("id", &deck)
                    .one::<Head>()
                    .await
                    .map_err(|e| e.to_string())?
                else {
                    return Ok(None);
                };
                let slides: Vec<Row> = Q::from("presentation_slides")
                    .select("id,canvas,hamper_id,product_id")
                    .eq("presentation_id", &deck)
                    .order("position")
                    .rows()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Some(Loaded { title: head.title, slides }))
            }
            .await;
            loaded.set(Some(res));
        });
    }
    let ready = RwSignal::new_local(None::<StudioConfig>);
    let (deck_s, slide_s) = (StoredValue::new(deck), StoredValue::new(slide));
    Effect::new(move |_| {
        if ready.with_untracked(Option::is_some) {
            return;
        }
        let (Some(Ok(state)), Some(products)) = (loaded.get(), catalog.data.get()) else { return };
        let (deck, slide) = (deck_s.get_value(), slide_s.get_value());
        let Some(l) = state else {
            return navigate.with_value(|nav| nav("/presentations", Default::default()));
        };
        let Some(index) = l.slides.iter().position(|s| s.id == slide) else {
            return navigate.with_value(|nav| nav(&deck_href(&deck), Default::default()));
        };
        let Some(row) = l.slides.get(index).cloned() else { return };
        let n = l.slides.len();
        let pages: Vec<PageLink> = l
            .slides
            .iter()
            .map(|s| PageLink {
                id: s.id.clone(),
                href: format!("{}/slides/{}", deck_href(&deck), s.id),
                canvas: parse_canvas(&s.canvas),
            })
            .collect();
        let products: Vec<PickerProduct> = products
            .into_iter()
            .map(|p| PickerProduct { id: p.id, code: p.code, name: p.name, image_url: p.image_url })
            .collect();
        spawn_local(async move {
            // "In this hamper": the slide's hamper's products, or its product.
            let related: Vec<String> = match (&row.hamper_id, &row.product_id) {
                (Some(h), _) => {
                    #[derive(Deserialize)]
                    struct P {
                        product_id: Option<String>,
                    }
                    let mut ids: Vec<String> = Q::from("hamper_items")
                        .select("product_id")
                        .eq("hamper_id", h)
                        .rows::<P>()
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|p| p.product_id)
                        .collect();
                    ids.sort();
                    ids.dedup();
                    ids
                }
                (None, Some(p)) => vec![p.clone()],
                _ => Vec::new(),
            };
            let (d2, s2) = (deck.clone(), slide.clone());
            let on_save: SaveFn = Rc::new(move |canvas, _| save_slide(d2.clone(), s2.clone(), canvas).boxed_local());
            ready.set(Some(StudioConfig {
                title: l.title.clone(),
                subtitle: format!("Slide {} of {n}", index + 1),
                back_href: deck_href(&deck),
                download_name: format!("{} - slide {}", l.title, index + 1),
                initial: parse_canvas(&row.canvas),
                products,
                hamper_product_ids: related,
                on_save,
                save_image: false,
                saved_message: "Slide saved.".into(),
                upload_folder: format!("presentations/{deck}"),
                pages,
                page_id: Some(slide),
            }));
        });
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
}
