//! The design studio (port of `components/studio/*`): the hamper image
//! designer and the presentation slide editor share it. Documents are
//! `lattice_core::canvas`; drawing is Canvas 2D from Rust (`render`).

mod editor;
mod magic;
mod panels;
pub mod render;

pub use editor::{PageLink, SaveFn, Studio, StudioConfig};
use lattice_core::api::ApiError;
use lattice_core::canvas::deck::{Photo, Size};
use lattice_core::canvas::{Canvas, Shape, fonts_href};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlImageElement;

/// A product the Products panel offers.
#[derive(Clone, Debug, PartialEq)]
pub struct PickerProduct {
    pub id: String,
    pub code: String,
    pub name: String,
    pub image_url: Option<String>,
}

pub fn uuid() -> String {
    window().crypto().map(|c| c.random_uuid()).unwrap_or_else(|_| format!("l{}", js_sys::Date::now() as u64))
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

enum Img {
    Loading(Vec<js_sys::Function>),
    Ready(HtmlImageElement),
    Failed,
}

thread_local! {
    static IMAGES: RefCell<HashMap<String, Img>> = RefCell::new(HashMap::new());
    static TICK: ArcRwSignal<u32> = ArcRwSignal::new(0);
}

/// Bumped whenever an image (or the web fonts) finish loading, so drawings refresh.
pub fn images_tick() -> ArcRwSignal<u32> {
    TICK.with(Clone::clone)
}

fn bump() {
    TICK.with(|t| t.update(|n| *n += 1));
}

/// A loaded image, or None while it loads (or if it failed); asking starts the load.
pub fn image(url: &str) -> Option<HtmlImageElement> {
    let ready = IMAGES.with(|m| match m.borrow().get(url) {
        Some(Img::Ready(i)) => Some(Some(i.clone())),
        Some(_) => Some(None),
        None => None,
    });
    match ready {
        Some(x) => x,
        None => {
            start_load(url, None);
            None
        }
    }
}

fn start_load(url: &str, waiter: Option<js_sys::Function>) {
    let fresh = IMAGES.with(|m| {
        let mut m = m.borrow_mut();
        match m.get_mut(url) {
            Some(Img::Loading(w)) => {
                w.extend(waiter);
                false
            }
            Some(_) => false,
            None => {
                m.insert(url.into(), Img::Loading(waiter.into_iter().collect()));
                true
            }
        }
    });
    if !fresh {
        return;
    }
    let Ok(img) = HtmlImageElement::new() else { return };
    // Remote photos must be CORS-clean, or exporting the drawing is blocked.
    if !url.starts_with("blob:") && !url.starts_with("data:") && url.starts_with("http") {
        img.set_cross_origin(Some("anonymous"));
    }
    let done = {
        let (img, url) = (img.clone(), url.to_string());
        move |ok: bool| {
            let waiters = IMAGES.with(|m| {
                let prev = m.borrow_mut().insert(url.clone(), if ok { Img::Ready(img.clone()) } else { Img::Failed });
                match prev {
                    Some(Img::Loading(w)) => w,
                    _ => Vec::new(),
                }
            });
            for w in waiters {
                let _ = w.call1(&JsValue::NULL, &JsValue::from_bool(ok));
            }
            bump();
        }
    };
    let ok = done.clone();
    img.set_onload(Some(Closure::once_into_js(move || ok(true)).unchecked_ref()));
    img.set_onerror(Some(Closure::once_into_js(move || done(false)).unchecked_ref()));
    img.set_src(url);
}

/// Waits for an image (15 s at most), as `loadImage` did.
pub async fn load_image(url: &str) -> Result<HtmlImageElement, ApiError> {
    if let Some(i) = image(url) {
        return Ok(i);
    }
    let failed = IMAGES.with(|m| matches!(m.borrow().get(url), Some(Img::Failed)));
    if failed {
        return Err(ApiError::Invalid(format!("Could not load {url}")));
    }
    let url_owned = url.to_string();
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        start_load(&url_owned, Some(resolve.clone()));
        let r = resolve.clone();
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
            Closure::once_into_js(move || {
                let _ = r.call1(&JsValue::NULL, &JsValue::from_bool(false));
            })
            .unchecked_ref(),
            15000,
        );
    });
    let _ = JsFuture::from(promise).await;
    image(url).ok_or_else(|| ApiError::Invalid(format!("Could not load {url}")))
}

/// Adds the Google Fonts stylesheet once.
pub fn ensure_font_stylesheet() {
    let doc = document();
    if doc.query_selector("link[data-studio-fonts]").ok().flatten().is_some() {
        return;
    }
    if let Ok(link) = doc.create_element("link") {
        let _ = link.set_attribute("rel", "stylesheet");
        let _ = link.set_attribute("href", &fonts_href());
        let _ = link.set_attribute("data-studio-fonts", "");
        if let Some(head) = doc.head() {
            let _ = head.append_child(&link);
        }
    }
}

/// Canvas text doesn't start web-font downloads by itself, so ask for each face in use.
pub async fn load_fonts(c: &Canvas) {
    ensure_font_stylesheet();
    let mut faces: Vec<String> = Vec::new();
    for l in &c.layers {
        if let Shape::Text(t) = &l.shape {
            let f = format!("{} 40px \"{}\"", t.font_style.css(), t.font_family);
            if !faces.contains(&f) {
                faces.push(f);
            }
        }
    }
    let fonts = document().fonts();
    for f in faces {
        let _ = JsFuture::from(fonts.load(&f)).await;
    }
}

const LOGO_PATH: &str = "brand/deck-logo.png";
const LOGO_SIZE: Size = Size { width: 1600.0, height: 887.0 };

/// The Lattice Lane logo for designs. It lives in storage so every design
/// keeps a stable URL; copied from the app's own asset the first time.
pub async fn brand_logo() -> Option<Photo> {
    let url: String = crate::tauri::invoke(
        "storage_public_url",
        &serde_json::json!({ "bucket": crate::db::PHOTOS, "path": LOGO_PATH }),
    )
    .await
    .ok()?;
    let there = crate::db::list_files("brand").await.ok()?.iter().any(|p| p == LOGO_PATH);
    if !there {
        let img = load_image("/assets/brand/deck-logo.png").await.ok()?;
        let (c, ctx) = render::new_canvas(img.natural_width(), img.natural_height())?;
        ctx.draw_image_with_html_image_element(&img, 0.0, 0.0).ok()?;
        let blob = crate::photos::to_blob(&c, "image/png", None).await.ok()??;
        let bytes = crate::photos::blob_bytes(&blob).await.ok()?;
        if let Err(e) = crate::db::upload(crate::db::PHOTOS, LOGO_PATH, &bytes, "image/png", false).await {
            let msg = e.to_string().to_lowercase();
            if !msg.contains("exist") && !msg.contains("duplicate") {
                return None;
            }
        }
    }
    Some(Photo { url, size: LOGO_SIZE })
}

/// The design as a small picture, redrawn when it changes.
#[component]
pub fn Thumb(#[prop(into)] canvas: Signal<Canvas>, width: f64, #[prop(optional)] class: &'static str) -> impl IntoView {
    let src = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let c = canvas.get();
        leptos::task::spawn_local(async move {
            src.set(render::render_data_url(&c, width * 2.0 / c.width).await);
        });
    });
    let ratio = move || canvas.with(|c| format!("width: {width}px; aspect-ratio: {} / {}", c.width, c.height));
    view! {
        <div class=format!("overflow-hidden bg-[#e3ddcc] {class}") style=ratio>
            {move || src.get().map(|s| view! { <img src=s alt="" class="block h-full w-full" /> })}
        </div>
    }
}
