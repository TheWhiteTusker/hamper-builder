//! Magic layers in the studio (port of `run-magic-layers.ts`,
//! `use-magic-layers.ts`, `magic-tools.tsx`): splits an image layer into a
//! filled-in background, a cut-out per object and an editable text layer per
//! line, placed exactly over the original. The models run natively
//! (`magic_analyze`); the pixel work here uses the full-size original.

use super::editor::Ed;
use super::render::new_canvas;
use super::{load_image, uuid};
use lattice_core::canvas::magic::{
    FramePlacer, LineBox, LowMask, Rect, alpha_for, dilate, fill_holes, fit_font_size, source_box, text_style,
};
use lattice_core::canvas::{Align, Base, Decoration, Fill, Fit, FontStyle, Image, Layer, Shape, Text};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::Clamped;

/// Detected text is set in Inter; it's restyled like any other text.
const FONT: &str = "Inter";
/// Per computer: the models are heavy, so each machine decides for itself.
const KEY: &str = "studio.magicLayers";

#[derive(Clone, Debug, Deserialize)]
pub struct Step {
    pub label: String,
    pub progress: Option<f64>,
}

#[derive(Deserialize)]
struct OcrLine {
    text: String,
    #[serde(rename = "box")]
    line: LineBox,
    row_height: f64,
}

#[derive(Deserialize)]
struct MaskOut {
    w: usize,
    h: usize,
    score: f32,
    logits_q: String,
}

#[derive(Deserialize)]
struct Analysis {
    lines: Vec<OcrLine>,
    masks: Vec<MaskOut>,
    scale: f64,
}

/// Machines with little memory or few cores start with Magic layers off.
pub fn looks_weak() -> bool {
    let nav = window().navigator();
    let memory = js_sys::Reflect::get(&nav, &"deviceMemory".into()).ok().and_then(|v| v.as_f64()).unwrap_or(8.0);
    memory < 4.0 || nav.hardware_concurrency() < 4.0
}

pub fn read_enabled() -> bool {
    let saved = window().local_storage().ok().flatten().and_then(|s| s.get_item(KEY).ok().flatten());
    saved.map_or_else(|| !looks_weak(), |v| v == "on")
}

pub fn write_enabled(on: bool) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(KEY, if on { "on" } else { "off" });
    }
}

fn decode_mask(m: &MaskOut) -> LowMask {
    let bin = window().atob(&m.logits_q).unwrap_or_default();
    let logits = bin.chars().map(|c| f32::from((c as u32 as u8) as i8) / 4.0).collect();
    LowMask { logits, w: m.w, h: m.h, score: m.score }
}

async fn png(c: &web_sys::HtmlCanvasElement) -> Result<Vec<u8>, String> {
    let blob = crate::photos::to_blob(c, "image/png", None)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Could not render a layer.")?;
    crate::photos::blob_bytes(&blob).await.map_err(|e| e.to_string())
}

/// The layers that replace `parent`; empty when there's nothing to separate.
// Pixel indices stay within the image (boxes are clamped to it).
#[allow(clippy::indexing_slicing)]
async fn split(ed: Ed, parent: Layer, steps: RwSignal<Option<Step>>) -> Result<Vec<Layer>, String> {
    let Shape::Image(frame) = &parent.shape else { return Ok(Vec::new()) };
    steps.set(Some(Step { label: "Loading image".into(), progress: None }));
    let img = load_image(&frame.url).await.map_err(|e| e.to_string())?;
    let (w, h) = (img.natural_width(), img.natural_height());
    let (wu, hu) = (w as usize, h as usize);
    let (src, sctx) = new_canvas(w, h).ok_or("No canvas")?;
    sctx.draw_image_with_html_image_element(&img, 0.0, 0.0).map_err(|_| "Could not draw the image")?;
    let mut original = sctx
        .get_image_data(0.0, 0.0, f64::from(w), f64::from(h))
        .map_err(|_| "Export blocked by an image from another website.")?
        .data()
        .0;
    let bytes = png(&src).await?;
    let analysis: Analysis = crate::tauri::invoke_raw("magic_analyze", &bytes, &[]).await.map_err(|e| e.to_string())?;
    if analysis.lines.is_empty() && analysis.masks.is_empty() {
        return Ok(Vec::new());
    }
    steps.set(Some(Step { label: "Separating layers".into(), progress: None }));
    // Read text colours before anything is painted over.
    let styles: Vec<(String, bool)> = analysis.lines.iter().map(|l| text_style(&original, wu, l.line)).collect();
    let mut text_hole = vec![false; wu * hu];
    for l in &analysis.lines {
        let b = l.line;
        for y in b.y0.saturating_sub(3)..(b.y1 + 3).min(hu) {
            for x in b.x0.saturating_sub(3)..(b.x1 + 3).min(wu) {
                text_hole[y * wu + x] = true;
            }
        }
    }
    let mut hole = text_hole.clone();
    let mut cutouts: Vec<(Rect, web_sys::HtmlCanvasElement)> = Vec::new();
    for m in analysis.masks.iter().map(decode_mask) {
        let Some(b) = source_box(&m, analysis.scale, wu, hu, 2) else { continue };
        if b.w == 0 || b.h == 0 {
            continue;
        }
        let alpha = alpha_for(&m, analysis.scale, b);
        let mut px = sctx
            .get_image_data(b.x as f64, b.y as f64, b.w as f64, b.h as f64)
            .map_err(|_| "Could not read the image")?
            .data()
            .0;
        // Text printed on the object becomes its own layer, so lift it off the cut-out too.
        let on_piece: Vec<bool> = (0..b.h)
            .flat_map(|y| (0..b.w).map(move |x| (x, y)))
            .map(|(x, y)| text_hole[(b.y + y) * wu + b.x + x])
            .collect();
        if on_piece.iter().any(|v| *v) {
            fill_holes(&mut px, b.w, b.h, &on_piece);
        }
        for (i, a) in alpha.iter().enumerate() {
            if let Some(p) = px.get_mut(i * 4 + 3) {
                *p = (*p).min(*a);
            }
            if *a > 64 {
                hole[(b.y + i / b.w) * wu + b.x + i % b.w] = true;
            }
        }
        let (piece, pctx) = new_canvas(b.w as u32, b.h as u32).ok_or("No canvas")?;
        let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(&px), b.w as u32, b.h as u32)
            .map_err(|_| "Could not process")?;
        pctx.put_image_data(&data, 0.0, 0.0).map_err(|_| "Could not process")?;
        cutouts.push((b, piece));
    }
    // Biggest first, so smaller objects sit on top of the ones they overlap.
    cutouts.sort_by_key(|(b, _)| std::cmp::Reverse(b.w * b.h));
    fill_holes(&mut original, wu, hu, &dilate(&hole, wu, hu, 3));
    let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(&original), w, h)
        .map_err(|_| "Could not process")?;
    sctx.put_image_data(&data, 0.0, 0.0).map_err(|_| "Could not process")?;

    steps.set(Some(Step { label: "Saving layers".into(), progress: None }));
    let folder = ed.config.with_value(|c| c.upload_folder.clone());
    let upload = |bytes: Vec<u8>| {
        let folder = folder.clone();
        async move {
            let path = crate::photos::asset_path(&folder, "png");
            crate::db::upload(crate::db::PHOTOS, &path, &bytes, "image/png", false)
                .await
                .map_err(|e| format!("Upload failed: {e}"))
        }
    };
    let bg_url = upload(png(&src).await?).await?;
    let mut piece_urls = Vec::new();
    for (_, c) in &cutouts {
        piece_urls.push(upload(png(c).await?).await?);
    }

    let place = FramePlacer::new(&parent.base, frame, f64::from(w), f64::from(h));
    let image_at = |url: String, name: &str, b: Rect| Layer {
        base: place.base_at(uuid(), b.x as f64, b.y as f64, name),
        shape: Shape::Image(Image {
            product_id: None,
            url,
            width: b.w as f64 * place.kx,
            height: b.h as f64 * place.ky,
            fit: Fit::Stretch,
        }),
    };
    let mut out = vec![image_at(bg_url, "Background", Rect { x: 0, y: 0, w: wu, h: hu })];
    for (i, ((b, _), url)) in cutouts.iter().zip(piece_urls).enumerate() {
        out.push(image_at(url, &format!("Object {}", i + 1), *b));
    }
    super::ensure_font_stylesheet();
    for f in [format!("100px \"{FONT}\""), format!("bold 100px \"{FONT}\"")] {
        let _ = wasm_bindgen_futures::JsFuture::from(document().fonts().load(&f)).await;
    }
    let (_, measure) = new_canvas(1, 1).ok_or("No canvas")?;
    for (l, (color, bold)) in analysis.lines.iter().zip(styles) {
        measure.set_font(&format!("{}100px \"{FONT}\"", if bold { "bold " } else { "" }));
        let at100 = measure.measure_text(&l.text).map(|m| m.width()).unwrap_or(0.0);
        let box_w = (l.line.x1 - l.line.x0) as f64;
        let size = fit_font_size(l.row_height, box_w, at100);
        let width = box_w.max(at100 * size / 100.0) * 1.05;
        // Centre the line box on the ink, so the retyped line sits where the old one was.
        let base: Base =
            place.base_at(uuid(), l.line.x0 as f64, (l.line.y0 + l.line.y1) as f64 / 2.0 - size / 2.0, &l.text);
        out.push(Layer {
            base: Base { name: None, ..base },
            shape: Shape::Text(Text {
                text: l.text.clone(),
                font_size: size * place.ky,
                font_family: FONT.into(),
                font_style: if bold { FontStyle::Bold } else { FontStyle::Normal },
                align: Align::Left,
                width: width * place.kx,
                letter_spacing: 0.0,
                line_height: 1.0,
                text_decoration: Decoration::None,
                fill: Fill::Solid { color },
            }),
        });
    }
    Ok(out)
}

/// Splits the image layer `id`, replacing it with its pieces in one undo step.
pub fn magic_split(ed: Ed, id: String, steps: RwSignal<Option<Step>>) {
    if steps.get_untracked().is_some() {
        return;
    }
    let Some(layer) = ed.with_canvas(|c| c.layers.iter().find(|l| l.base.id == id && l.kind() == "image").cloned())
    else {
        return;
    };
    steps.set(Some(Step { label: "Starting".into(), progress: None }));
    spawn_local(async move {
        let res = split(ed, layer, steps).await;
        let at = steps.get_untracked().map(|s| s.label).unwrap_or_default();
        steps.set(None);
        match res {
            Ok(layers) if layers.is_empty() => {
                ed.notify(false, "No text or separate objects were found in that image.")
            }
            Ok(layers) => {
                let ids: Vec<String> = layers.iter().map(|l| l.base.id.clone()).collect();
                let n = layers.len();
                ed.change(
                    |c| {
                        if let Some(i) = c.layers.iter().position(|l| l.base.id == id) {
                            c.layers.splice(i..=i, layers);
                        }
                    },
                    None,
                );
                ed.select_many(ids);
                ed.notify(
                    true,
                    format!("Split into {n} layers. Double-click text to edit it; Undo puts the original back."),
                );
            }
            Err(e) => ed.notify(
                false,
                format!("Magic layers failed at \"{at}\": {e}. Try again; downloads often succeed on a retry."),
            ),
        }
    });
}

/// The on/off switch for this computer, progress, and the Split button.
#[component]
pub fn MagicTools(ed: Ed, id: String) -> impl IntoView {
    let steps = ed.magic;
    let enabled = RwSignal::new(read_enabled());
    let open = RwSignal::new(false);
    let id = StoredValue::new(id);
    view! {
        <div class="relative">
            <button type="button" class="inline-flex h-8 items-center gap-1 rounded-md px-2 text-[12.5px] hover:bg-(--st-hover)" on:click=move |_| open.update(|o| *o = !*o)>
                {move || if steps.get().is_some() { "Splitting…" } else { "✦ Magic layers" }}
            </button>
            <Show when=move || open.get()>
                <div class="absolute top-9 left-0 z-50 w-72 space-y-3 rounded-lg border border-(--st-line) bg-(--st-panel) p-3 shadow-xl">
                    <label class="flex cursor-pointer items-center gap-2 text-[12px] select-none">
                        <input type="checkbox" prop:checked=enabled disabled=move || steps.get().is_some()
                            on:change=move |e| { let on = event_target_checked(&e); enabled.set(on); write_enabled(on); } />
                        "Enable Magic layers on this computer"
                    </label>
                    <p class="text-[11.5px] leading-snug text-(--st-muted)">
                        {move || if enabled.get() {
                            "Splits this image into a background, a cut-out for each object and editable text. Runs on this computer; the first run downloads the AI models (about 52 MB, then kept). Undo brings the original back.".to_string()
                        } else {
                            format!("Off on this computer, so nothing is downloaded.{}", if looks_weak() { " It started off because this computer looks slow for it; you can still turn it on." } else { "" })
                        }}
                    </p>
                    {move || steps.get().map(|s| view! {
                        <div class="space-y-1">
                            <p class="text-[12px]">{format!("{}…", s.label)}</p>
                            {s.progress.map(|p| view! {
                                <div class="h-1.5 overflow-hidden rounded bg-(--st-line)">
                                    <div class="h-full bg-(--st-accent)" style=format!("width: {}%", (p * 100.0).round())></div>
                                </div>
                            })}
                        </div>
                    })}
                    <button type="button" class="w-full rounded-md bg-(--st-accent) px-3 py-1.5 text-[12.5px] font-medium text-(--st-on-accent) disabled:opacity-50"
                        disabled=move || !enabled.get() || steps.get().is_some() on:click=move |_| magic_split(ed, id.get_value(), steps)>
                        {move || if steps.get().is_some() { "Splitting…" } else { "Split into layers" }}
                    </button>
                </div>
            </Show>
        </div>
    }
}
