//! `/presentations/<id>/print`: every slide drawn full size, then the print
//! dialog; "Save as PDF" there gives one page per slide at the slide's shape
//! (port of `print/print-deck.tsx`).

use super::deck_href;
use crate::db::Q;
use crate::pages::quotes::print_page;
use crate::studio::render;
use lattice_core::canvas::deck::PX_PER_INCH;
use lattice_core::canvas::{Canvas, parse_canvas};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use serde_json::Value;

#[component]
pub fn PrintDeck() -> impl IntoView {
    let id = use_params_map().with_untracked(|p| p.get("id")).unwrap_or_default();
    let title = RwSignal::new(String::new());
    let slides = RwSignal::new(Vec::<Canvas>::new());
    let images = RwSignal::new(Vec::<Option<String>>::new());
    let failed = RwSignal::new(false);
    let loading = RwSignal::new(true);
    {
        let id = id.clone();
        spawn_local(async move {
            #[derive(Deserialize)]
            struct Head {
                title: String,
            }
            #[derive(Deserialize)]
            struct Row {
                canvas: Value,
            }
            let head = Q::from("presentations").select("title").eq("id", &id).one::<Head>().await.ok().flatten();
            let rows: Vec<Row> = Q::from("presentation_slides").select("canvas").eq("presentation_id", &id).order("position").rows().await.unwrap_or_default();
            title.set(head.map(|h| h.title).unwrap_or_default());
            let canvases: Vec<Canvas> = rows.iter().map(|r| parse_canvas(&r.canvas)).collect();
            images.set(vec![None; canvases.len()]);
            slides.set(canvases.clone());
            loading.set(false);
            for (i, c) in canvases.iter().enumerate() {
                match render::render(c, 1.0, false).await.ok().and_then(|(el, _)| el.to_data_url().ok()) {
                    Some(url) => images.update(|v| {
                        if let Some(slot) = v.get_mut(i) {
                            *slot = Some(url);
                        }
                    }),
                    None => return failed.set(true),
                }
            }
            // Becomes the suggested PDF file name.
            document().set_title(&title.get_untracked());
            set_timeout(print_page, std::time::Duration::from_millis(300));
        });
    }
    let done = move || images.with(|v| v.iter().filter(|x| x.is_some()).count());
    let ready = move || !slides.with(Vec::is_empty) && done() == slides.with(Vec::len);
    let page_css = move || {
        slides.with(|s| s.first().map(|c| {
            format!(
                "@media print {{ @page {{ size: {}in {}in; margin: 0; }} main {{ padding: 0 !important; max-width: none !important; }} .deck-page {{ width: 100vw; height: 100vh; margin: 0 !important; box-shadow: none !important; break-after: page; }} .deck-page:last-child {{ break-after: auto; }} }}",
                c.width / PX_PER_INCH,
                c.height / PX_PER_INCH
            )
        }))
    };
    let back = deck_href(&id);
    view! {
        {move || page_css().map(|css| view! { <style>{css}</style> })}
        <div class="no-print mb-4 flex flex-wrap items-center gap-3">
            <A href=back attr:class="btn-secondary">"Back"</A>
            <span class="text-sm text-(--color-muted)">
                {move || if failed.get() {
                    "A slide could not be drawn. An image from another website may be blocking it.".to_string()
                } else if loading.get() {
                    "Loading slides…".to_string()
                } else if ready() {
                    format!("{} slides ready. In the print dialog choose \"Save as PDF\".", slides.with(Vec::len))
                } else {
                    format!("Preparing slide {} of {}…", (done() + 1).min(slides.with(Vec::len)), slides.with(Vec::len))
                }}
            </span>
            <button type="button" class="btn-primary ml-auto" disabled=move || !ready() on:click=move |_| print_page()>"Print / PDF"</button>
        </div>
        <div class="space-y-6">
            {move || slides.get().into_iter().enumerate().map(|(i, c)| view! {
                <div class="deck-page mx-auto flex max-w-5xl items-center justify-center overflow-hidden bg-white shadow-md" style=format!("aspect-ratio: {} / {}", c.width, c.height)>
                    {move || match images.with(|v| v.get(i).cloned().flatten()) {
                        Some(src) => view! { <img src=src alt=format!("Slide {}", i + 1) class="h-full w-full object-contain" /> }.into_any(),
                        None => view! { <span class="text-sm text-(--color-muted)">{format!("Slide {}…", i + 1)}</span> }.into_any(),
                    }}
                </div>
            }).collect_view()}
        </div>
    }
}
