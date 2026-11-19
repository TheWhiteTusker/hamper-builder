//! The studio's stage (port of `canvas-stage.tsx` and `stage/*`): the design
//! with undo history and selection, pointer gestures (move with snapping,
//! resize and rotate handles, line handles, marquee, drawing, pan and zoom),
//! keyboard shortcuts, images, and saving.

use super::panels::{ContextMenu, ContextToolbar, Footer, LayersPanel, PageStrip, Rail, SidePanel, Tab, Topbar, TransformPanel};
use super::render::{self, Mat, client_rect, hit, node_mat, self_rect};
use super::{PickerProduct, images_tick, load_image, now_ms, uuid};
use futures::future::LocalBoxFuture;
use lattice_core::canvas::arrange::{self, Box as Rect, LayerMove, PageAlign, Snapped};
use lattice_core::canvas::history::History;
use lattice_core::canvas::{Base, Canvas, Curve, Image, Layer, Line, LineCap, Shape};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// Saves the design (and its PNG when `save_image`); Err is the message to show.
pub type SaveFn = Rc<dyn Fn(Canvas, Option<Vec<u8>>) -> LocalBoxFuture<'static, Result<(), String>>>;

/// A sibling page (a deck's slides) shown in a strip under the page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLink {
    pub id: String,
    pub href: String,
    pub canvas: Canvas,
}

#[derive(Clone)]
pub struct StudioConfig {
    pub title: String,
    pub subtitle: String,
    pub back_href: String,
    pub download_name: String,
    pub initial: Canvas,
    pub products: Vec<PickerProduct>,
    /// Products offered first in the Products panel ("In this hamper").
    pub hamper_product_ids: Vec<String>,
    pub on_save: SaveFn,
    pub save_image: bool,
    pub saved_message: String,
    /// Storage folder for images uploaded in the editor.
    pub upload_folder: String,
    pub pages: Vec<PageLink>,
    pub page_id: Option<String>,
}

pub const ACCENT: &str = "#54655b";
pub const GUIDE: &str = "#b8962e";
pub const LOCKED: &str = "#a8a28f";
pub const HANDLE_FILL: &str = "#faf8ee";
pub const ZOOM_MIN: f64 = 0.05;
pub const ZOOM_MAX: f64 = 4.0;
pub const DRAG_MIME: &str = "application/x-hamper-image";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Line,
    Curve,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct View {
    pub zoom: Option<f64>,
    pub pan_x: f64,
    pub pan_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchor {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    TopMid,
    RightMid,
    BottomMid,
    LeftMid,
}

impl Anchor {
    fn opposite(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
            Self::TopMid => Self::BottomMid,
            Self::RightMid => Self::LeftMid,
            Self::BottomMid => Self::TopMid,
            Self::LeftMid => Self::RightMid,
        }
    }
    fn corner(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight | Self::BottomRight | Self::BottomLeft)
    }
    /// Where on a box, as fractions of width and height.
    fn at(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::BottomRight => (1.0, 1.0),
            Self::BottomLeft => (0.0, 1.0),
            Self::TopMid => (0.5, 0.0),
            Self::RightMid => (1.0, 0.5),
            Self::BottomMid => (0.5, 1.0),
            Self::LeftMid => (0.0, 0.5),
        }
    }
}

const ALL_ANCHORS: [Anchor; 8] =
    [Anchor::TopLeft, Anchor::TopRight, Anchor::BottomRight, Anchor::BottomLeft, Anchor::TopMid, Anchor::RightMid, Anchor::BottomMid, Anchor::LeftMid];

/// What the transformer wraps: one layer (its own rotated box) or several (their combined box).
#[derive(Clone, Debug)]
enum Frame {
    Single { layer: Layer, m: Mat, r: Rect },
    Multi { ids: Vec<String>, b: Rect },
}

#[derive(Clone, Debug)]
enum Gesture {
    Pan { last: (f64, f64) },
    Drag { ids: Vec<String>, start: (f64, f64), origin: Vec<Layer>, rects: Vec<Rect>, others: Vec<Rect>, group: String },
    Resize { anchor: Anchor, origin: Vec<Layer>, frame: Frame, group: String },
    Rotate { origin: Vec<Layer>, frame: Frame, group: String },
    Handle { id: String, which: usize, group: String },
    Marquee { start: (f64, f64), keep: Vec<String> },
    Draw,
}

/// Everything the panels and toolbars can do to the design.
#[derive(Clone, Copy)]
pub struct Ed {
    pub hist: RwSignal<History<Canvas>>,
    pub saved: RwSignal<Canvas>,
    pub selected: RwSignal<Vec<String>>,
    pub editing: RwSignal<Option<String>>,
    pub menu: RwSignal<Option<(f64, f64)>>,
    pub clipboard: StoredValue<Vec<Layer>>,
    pub tool: RwSignal<Tool>,
    pub snap_right: RwSignal<bool>,
    pub view: RwSignal<View>,
    pub board: RwSignal<(f64, f64)>,
    guides: RwSignal<(Vec<f64>, Vec<f64>)>,
    hover: RwSignal<Option<Rect>>,
    marquee: RwSignal<Option<Rect>>,
    drawing: RwSignal<Option<((f64, f64), (f64, f64), Tool)>>,
    gesture: StoredValue<Option<Gesture>>,
    pub toast: RwSignal<Option<(bool, String)>>,
    pub saving: RwSignal<bool>,
    pub swap_id: RwSignal<Option<String>>,
    pub uploads: RwSignal<Vec<(String, String)>>,
    pub uploading: RwSignal<bool>,
    pub removing: RwSignal<bool>,
    pub space: RwSignal<bool>,
    pub tab: RwSignal<Option<Tab>>,
    pub config: StoredValue<StudioConfig, LocalStorage>,
    board_el: NodeRef<html::Div>,
}

fn same_ids(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.contains(x))
}

impl Ed {
    pub fn canvas(&self) -> Canvas {
        self.hist.with(|h| h.present.clone())
    }
    pub fn with_canvas<R>(&self, f: impl FnOnce(&Canvas) -> R) -> R {
        self.hist.with(|h| f(&h.present))
    }
    fn with_canvas_untracked<R>(&self, f: impl FnOnce(&Canvas) -> R) -> R {
        self.hist.with_untracked(|h| f(&h.present))
    }
    pub fn dirty(&self) -> bool {
        self.hist.with(|h| self.saved.with(|s| h.present != *s))
    }

    /// One edit; continuous edits share a `group` so they undo as one step.
    pub fn change(&self, f: impl FnOnce(&mut Canvas), group: Option<&str>) {
        self.hist.update(|h| {
            let mut c = h.present.clone();
            f(&mut c);
            h.record(c, group, now_ms());
        });
    }
    pub fn patch(&self, id: &str, f: impl FnOnce(&mut Layer), group: Option<&str>) {
        self.change(
            |c| {
                if let Some(l) = c.layers.iter_mut().find(|l| l.id() == id) {
                    f(l);
                }
            },
            group,
        );
    }
    /// For continuous controls: one undo step per control.
    pub fn tweak(&self, id: &str, key: &str, f: impl FnOnce(&mut Layer)) {
        self.patch(id, f, Some(&format!("{id}:{key}")));
    }
    pub fn patch_each(&self, ids: &[String], f: impl Fn(&mut Layer), group: Option<&str>) {
        self.change(|c| c.layers.iter_mut().filter(|l| ids.iter().any(|i| i == l.id())).for_each(&f), group);
    }
    fn shift(&self, offsets: &[(String, f64, f64)], group: Option<&str>) {
        self.change(
            |c| {
                for l in &mut c.layers {
                    if let Some((_, dx, dy)) = offsets.iter().find(|(i, _, _)| i == l.id()) {
                        l.base.x += dx;
                        l.base.y += dy;
                    }
                }
            },
            group,
        );
    }

    /// Selected layers, in z-order.
    pub fn selection(&self) -> Vec<Layer> {
        let ids = self.selected.get();
        self.with_canvas(|c| c.layers.iter().filter(|l| ids.contains(&l.base.id)).cloned().collect())
    }
    pub fn selected_one(&self) -> Option<Layer> {
        let s = self.selection();
        if s.len() == 1 { s.into_iter().next() } else { None }
    }
    pub fn select_many(&self, ids: Vec<String>) {
        if let Some(e) = self.editing.get_untracked()
            && !(ids.len() == 1 && ids[0] == e)
        {
            self.editing.set(None);
        }
        self.menu.set(None);
        self.selected.set(ids);
    }
    pub fn select(&self, id: Option<String>) {
        self.select_many(id.into_iter().collect());
    }
    pub fn toggle(&self, id: &str) {
        let mut ids = self.selected.get_untracked();
        if let Some(i) = ids.iter().position(|x| x == id) {
            ids.remove(i);
        } else {
            ids.push(id.to_string());
        }
        self.select_many(ids);
    }
    pub fn select_all(&self) {
        let ids = self.with_canvas_untracked(|c| c.layers.iter().filter(|l| l.base.visible && !l.base.locked).map(|l| l.base.id.clone()).collect());
        self.select_many(ids);
    }
    pub fn add_many(&self, layers: Vec<Layer>) {
        let ids = layers.iter().map(|l| l.base.id.clone()).collect();
        self.change(|c| c.layers.extend(layers), None);
        self.select_many(ids);
    }
    pub fn add(&self, layer: Layer) {
        self.add_many(vec![layer]);
    }
    pub fn remove(&self, ids: &[String]) {
        self.change(|c| c.layers.retain(|l| !ids.iter().any(|i| i == l.id())), None);
        self.select(None);
    }
    fn copy_of(l: &Layer) -> Layer {
        let mut c = l.clone();
        c.base.id = uuid();
        c.base.locked = false;
        c.base.x += 24.0;
        c.base.y += 24.0;
        c
    }
    pub fn duplicate(&self, ids: &[String]) {
        let copies = self.with_canvas_untracked(|c| c.layers.iter().filter(|l| ids.iter().any(|i| i == l.id())).map(Self::copy_of).collect());
        self.add_many(copies);
    }
    pub fn copy(&self) {
        let s = self.selection();
        if !s.is_empty() {
            self.clipboard.set_value(s);
        }
    }
    /// Each paste lands a little further along, like Canva.
    pub fn paste(&self) {
        let copies: Vec<Layer> = self.clipboard.with_value(|c| c.iter().map(Self::copy_of).collect());
        if copies.is_empty() {
            return;
        }
        self.clipboard.set_value(copies.clone());
        self.add_many(copies);
    }
    pub fn move_layers(&self, ids: &[String], m: LayerMove) {
        self.change(|c| c.layers = arrange::move_canvas_layers(&c.layers, ids, m), None);
    }
    pub fn reorder(&self, id: &str, to: usize) {
        self.change(
            |c| {
                if let Some(from) = c.layers.iter().position(|l| l.id() == id) {
                    c.layers = arrange::reorder_layer(&c.layers, from, to);
                }
            },
            None,
        );
    }
    pub fn undo(&self) {
        self.hist.update(History::undo);
    }
    pub fn redo(&self) {
        self.hist.update(History::redo);
    }

    /// Unlocked layers among `ids`, with their boxes on the page.
    fn movable(&self, ids: &[String]) -> Vec<(String, Rect)> {
        self.with_canvas_untracked(|c| c.layers.iter().filter(|l| ids.iter().any(|i| i == l.id()) && !l.base.locked).map(|l| (l.base.id.clone(), client_rect(l))).collect())
    }
    /// One layer aligns to the page; several align to each other.
    pub fn align(&self, ids: &[String], at: PageAlign) {
        let items = self.movable(ids);
        if items.is_empty() {
            return;
        }
        let (w, h) = self.with_canvas_untracked(|c| (c.width, c.height));
        let offs: Vec<(f64, f64)> = if items.len() == 1 {
            vec![arrange::align_to_page(items[0].1, w, h, at)]
        } else {
            arrange::align_boxes(&items.iter().map(|i| i.1).collect::<Vec<_>>(), at)
        };
        self.shift(&items.iter().zip(offs).map(|((id, _), (dx, dy))| (id.clone(), dx, dy)).collect::<Vec<_>>(), None);
    }
    pub fn distribute(&self, ids: &[String], horizontal: bool) {
        let items = self.movable(ids);
        let offs = arrange::distribute_boxes(&items.iter().map(|i| i.1).collect::<Vec<_>>(), horizontal);
        self.shift(&items.iter().zip(offs).map(|((id, _), (dx, dy))| (id.clone(), dx, dy)).collect::<Vec<_>>(), None);
    }
    /// Mirrors a layer about its own centre.
    pub fn flip(&self, id: &str, horizontal: bool) {
        let Some(l) = self.with_canvas_untracked(|c| c.layers.iter().find(|l| l.id() == id).cloned()) else { return };
        let before = client_rect(&l);
        let mut next = l.clone();
        if horizontal {
            next.base.scale_x = -next.base.scale_x;
        } else {
            next.base.scale_y = -next.base.scale_y;
        }
        let after = client_rect(&next);
        let (dx, dy) = (before.x + before.width / 2.0 - (after.x + after.width / 2.0), before.y + before.height / 2.0 - (after.y + after.height / 2.0));
        self.patch(
            id,
            |x| {
                x.base.scale_x = next.base.scale_x;
                x.base.scale_y = next.base.scale_y;
                x.base.x += dx;
                x.base.y += dy;
            },
            None,
        );
    }
    /// Scales a layer down to fit the page (never up) and centres it.
    pub fn fit_to_page(&self, id: &str) {
        let Some((l, w, h)) = self.with_canvas_untracked(|c| c.layers.iter().find(|l| l.id() == id).cloned().map(|l| (l, c.width, c.height))) else { return };
        let k = arrange::fit_scale(client_rect(&l), w, h, 0.9);
        let mut next = l.clone();
        next.base.scale_x *= k;
        next.base.scale_y *= k;
        let after = client_rect(&next);
        self.patch(
            id,
            |x| {
                x.base.scale_x = next.base.scale_x;
                x.base.scale_y = next.base.scale_y;
                x.base.x += (w - after.width) / 2.0 - after.x;
                x.base.y += (h - after.height) / 2.0 - after.y;
            },
            None,
        );
    }

    pub fn notify(&self, ok: bool, text: impl Into<String>) {
        let text = text.into();
        self.toast.set(Some((ok, text.clone())));
        let toast = self.toast;
        set_timeout(
            move || toast.update(|t| if t.as_ref().is_some_and(|(_, x)| *x == text) { *t = None }),
            std::time::Duration::from_millis(if ok { 3500 } else { 7000 }),
        );
    }

    // ------------------------------------------------------------ view

    pub fn fit(&self) -> f64 {
        let (bw, bh) = self.board.get();
        let (w, h) = self.with_canvas(|c| (c.width, c.height));
        if bw <= 0.0 { 0.3 } else { ZOOM_MIN.max(((bw - 96.0) / w).min((bh - 96.0) / h)) }
    }
    pub fn zoom(&self) -> f64 {
        self.view.get().zoom.unwrap_or_else(|| self.fit())
    }
    /// The page's top-left on the board, and the zoom.
    pub fn origin(&self) -> (f64, f64, f64) {
        let (bw, bh) = self.board.get();
        let (w, h) = self.with_canvas(|c| (c.width, c.height));
        let v = self.view.get();
        let z = self.zoom();
        ((bw - w * z) / 2.0 + v.pan_x, (bh - h * z) / 2.0 + v.pan_y, z)
    }
    /// Zoom keeping the page point under `anchor` (board coords) still.
    pub fn zoom_to(&self, next: impl Fn(f64) -> f64, anchor: Option<(f64, f64)>) {
        let (bw, bh) = self.board.get_untracked();
        let (w, h) = self.with_canvas_untracked(|c| (c.width, c.height));
        let fit = self.fit();
        self.view.update(|v| {
            let z = v.zoom.unwrap_or(fit);
            let nz = next(z).clamp(ZOOM_MIN, ZOOM_MAX);
            let (ax, ay) = anchor.unwrap_or((bw / 2.0, bh / 2.0));
            let px = (ax - ((bw - w * z) / 2.0 + v.pan_x)) / z;
            let py = (ay - ((bh - h * z) / 2.0 + v.pan_y)) / z;
            *v = View { zoom: Some(nz), pan_x: ax - px * nz - (bw - w * nz) / 2.0, pan_y: ay - py * nz - (bh - h * nz) / 2.0 };
        });
    }
    pub fn zoom_fit(&self) {
        self.view.set(View::default());
    }
    pub fn recenter(&self) {
        let z = self.zoom();
        self.view.set(View { zoom: Some(z), pan_x: 0.0, pan_y: 0.0 });
    }
    pub fn resize_page(&self, w: f64, h: f64) {
        self.change(
            |c| {
                c.width = w;
                c.height = h;
            },
            None,
        );
        self.zoom_fit();
    }

    fn board_point(&self, client_x: f64, client_y: f64) -> (f64, f64) {
        self.board_el.get_untracked().map_or((client_x, client_y), |el| {
            let r = el.get_bounding_client_rect();
            (client_x - r.left(), client_y - r.top())
        })
    }
    fn page_point(&self, client_x: f64, client_y: f64) -> (f64, f64) {
        let (bx, by) = self.board_point(client_x, client_y);
        let (ox, oy, z) = self.origin();
        ((bx - ox) / z, (by - oy) / z)
    }

    // ---------------------------------------------------------- images

    /// A new image at `at` (or the page centre), scaled to fit within `share` of the page.
    fn image_layer(&self, img: &web_sys::HtmlImageElement, url: &str, name: &str, product_id: Option<String>, at: Option<(f64, f64)>, share: f64) -> Layer {
        let (w, h) = self.with_canvas_untracked(|c| (c.width, c.height));
        let (nw, nh) = (f64::from(img.natural_width()).max(1.0), f64::from(img.natural_height()).max(1.0));
        let k = 1f64.min(w.min(h) * share / nw.max(nh));
        let (iw, ih) = (nw * k, nh * k);
        let (cx, cy) = at.unwrap_or((w / 2.0, h / 2.0));
        Layer {
            base: Base { name: Some(name.into()), ..Base::at(uuid(), cx - iw / 2.0, cy - ih / 2.0) },
            shape: Shape::Image(Image { product_id, url: url.into(), width: iw, height: ih, fit: lattice_core::canvas::Fit::Stretch }),
        }
    }
    /// An image that isn't a product photo (an upload, the logo).
    pub fn add_image(&self, url: String, name: String, at: Option<(f64, f64)>) {
        let ed = *self;
        spawn_local(async move {
            match load_image(&url).await {
                Ok(img) => ed.add(ed.image_layer(&img, &url, &name, None, at, 0.45)),
                Err(_) => ed.notify(false, "That image could not be loaded."),
            }
        });
    }
    pub fn add_logo(&self) {
        let ed = *self;
        spawn_local(async move {
            let Some(logo) = super::brand_logo().await else { return ed.notify(false, "Could not load the Lattice Lane logo.") };
            match load_image(&logo.url).await {
                Ok(img) => ed.add(ed.image_layer(&img, &logo.url, "Logo", None, None, 0.3)),
                Err(_) => ed.notify(false, "Could not load the Lattice Lane logo."),
            }
        });
    }
    /// Adds a product photo, or replaces the image being swapped.
    pub fn place_image(&self, p: PickerProduct, url: String, at: Option<(f64, f64)>) {
        let ed = *self;
        spawn_local(async move {
            let Ok(img) = load_image(&url).await else {
                return ed.notify(false, "That image could not be loaded. If it's an external link, upload it to the product instead.");
            };
            let swapping = ed.swap_id.get_untracked().and_then(|id| ed.with_canvas_untracked(|c| c.layers.iter().find(|l| l.id() == id && l.kind() == "image").map(|l| l.base.id.clone())));
            if let Some(id) = swapping {
                let (nw, nh) = (f64::from(img.natural_width()).max(1.0), f64::from(img.natural_height()).max(1.0));
                ed.patch(
                    &id,
                    |l| {
                        l.base.name = Some(p.name.clone());
                        if let Shape::Image(i) = &mut l.shape {
                            i.url = url.clone();
                            i.product_id = Some(p.id.clone());
                            i.height = i.width * nh / nw;
                        }
                    },
                    None,
                );
                ed.swap_id.set(None);
                ed.select(Some(id));
                return;
            }
            ed.add(ed.image_layer(&img, &url, &p.name, Some(p.id.clone()), at, 0.45));
        });
    }
    async fn upload_file(&self, file: &web_sys::File) -> Result<String, String> {
        if !file.type_().starts_with("image/") {
            return Err("That file is not an image.".into());
        }
        let (bytes, mime, ext) = crate::photos::optimize(file).await.map_err(|e| e.to_string())?;
        let folder = self.config.with_value(|c| c.upload_folder.clone());
        let path = crate::photos::asset_path(&folder, &ext);
        crate::db::upload(crate::db::PHOTOS, &path, &bytes, &mime, false).await.map_err(|e| format!("Storage upload failed: {e}"))
    }
    /// Uploads images and adds each to the page as its own layer.
    pub fn upload_images(&self, files: Vec<web_sys::File>) {
        if files.is_empty() {
            return;
        }
        let ed = *self;
        ed.uploading.set(true);
        spawn_local(async move {
            let (w, h) = ed.with_canvas_untracked(|c| (c.width, c.height));
            let (mut added, mut done, mut failed) = (Vec::new(), Vec::new(), Vec::new());
            for (i, file) in files.iter().enumerate() {
                match ed.upload_file(file).await {
                    Ok(url) => {
                        done.push((url.clone(), file.name()));
                        let off = i as f64 * 40.0;
                        match load_image(&url).await {
                            Ok(img) => added.push(ed.image_layer(&img, &url, &file.name(), None, Some((w / 2.0 + off, h / 2.0 + off)), 0.45)),
                            Err(_) => failed.push(format!("{}: uploaded but could not be displayed", file.name())),
                        }
                    }
                    Err(e) => failed.push(format!("{}: {e}", file.name())),
                }
            }
            ed.uploading.set(false);
            let n = added.len();
            ed.uploads.update(|u| {
                let mut next = done.clone();
                next.append(u);
                *u = next;
            });
            if n > 0 {
                ed.add_many(added);
            }
            if !failed.is_empty() {
                ed.notify(false, failed.join(" · "));
            } else if !done.is_empty() {
                ed.notify(true, format!("Added {n} image{}.", if n == 1 { "" } else { "s" }));
            }
        });
    }
    pub fn upload_background(&self, file: web_sys::File) {
        let ed = *self;
        ed.uploading.set(true);
        spawn_local(async move {
            let res = ed.upload_file(&file).await;
            ed.uploading.set(false);
            match res {
                Ok(url) => {
                    let _ = load_image(&url).await;
                    ed.change(|c| c.background.image_url = Some(url), None);
                    ed.notify(true, "Background image updated.");
                }
                Err(e) => ed.notify(false, e),
            }
        });
    }
    /// Makes a product photo's plain background transparent and saves the
    /// cut-out to the product's images.
    pub fn cut_out(&self, id: String, tolerance: f64) {
        let ed = *self;
        let Some((url, product_id)) = ed.with_canvas_untracked(|c| {
            c.layers.iter().find(|l| l.id() == id).and_then(|l| match &l.shape {
                Shape::Image(i) => i.product_id.clone().map(|p| (i.url.clone(), p)),
                _ => None,
            })
        }) else {
            return;
        };
        ed.removing.set(true);
        spawn_local(async move {
            let res: Result<String, String> = async {
                let img = load_image(&url).await.map_err(|e| e.to_string())?;
                let (nw, nh) = (f64::from(img.natural_width()), f64::from(img.natural_height()));
                let k = 1f64.min(2000.0 / nw.max(nh));
                let (w, h) = ((nw * k).round() as u32, (nh * k).round() as u32);
                let (c, ctx) = render::new_canvas(w, h).ok_or("No canvas")?;
                ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, f64::from(w), f64::from(h)).map_err(|_| "Could not draw")?;
                let data = ctx.get_image_data(0.0, 0.0, f64::from(w), f64::from(h)).map_err(|_| "Export blocked by an image from another website.")?;
                let mut px = data.data().0;
                lattice_core::canvas::cutout::remove_background(&mut px, w as usize, h as usize, tolerance);
                let out = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&px), w, h).map_err(|_| "Could not process")?;
                ctx.put_image_data(&out, 0.0, 0.0).map_err(|_| "Could not process")?;
                let blob = crate::photos::to_blob(&c, "image/png", None).await.map_err(|e| e.to_string())?.ok_or("render failed")?;
                let opts = web_sys::FilePropertyBag::new();
                opts.set_type("image/png");
                let parts = js_sys::Array::of1(&blob);
                let file = web_sys::File::new_with_blob_sequence_and_options(&parts, "cutout.png", &opts).map_err(|_| "Could not process")?;
                let saved = crate::photos::upload_product_image(&file, &product_id, None, false).await.map_err(|e| e.to_string())?;
                Ok(saved.url)
            }
            .await;
            ed.removing.set(false);
            match res {
                Ok(new_url) => {
                    ed.patch(
                        &id,
                        |l| {
                            if let Shape::Image(i) = &mut l.shape {
                                i.url = new_url.clone();
                            }
                        },
                        None,
                    );
                    ed.notify(true, "Background removed. The cut-out is saved to the product's images.");
                }
                Err(e) => ed.notify(false, if e.is_empty() { "Could not process that image.".to_string() } else { e }),
            }
        });
    }

    // ------------------------------------------------------------ save

    /// Saves the design (and its PNG when asked). Resolves false if it didn't save.
    pub async fn save(&self) -> bool {
        if self.saving.get_untracked() {
            return false;
        }
        self.saving.set(true);
        self.editing.set(None);
        let canvas = self.hist.with_untracked(|h| h.present.clone());
        let (save_image, on_save, msg) = self.config.with_value(|c| (c.save_image, c.on_save.clone(), c.saved_message.clone()));
        let png = if save_image {
            match render::render_png(&canvas, 1.0, false).await {
                Ok((bytes, _)) => Some(bytes),
                Err(e) => {
                    self.saving.set(false);
                    self.notify(false, e.to_string());
                    return false;
                }
            }
        } else {
            None
        };
        let res = on_save(canvas.clone(), png).await;
        self.saving.set(false);
        match res {
            Ok(()) => {
                self.saved.set(canvas);
                self.notify(true, msg);
                true
            }
            Err(e) => {
                self.notify(false, e);
                false
            }
        }
    }
    pub fn download(&self) {
        let ed = *self;
        spawn_local(async move {
            let canvas = ed.canvas();
            let name = ed.config.with_value(|c| c.download_name.clone());
            match render::render_png(&canvas, 1.0, false).await {
                Ok((bytes, _)) => {
                    let saved: Result<Option<String>, _> = crate::tauri::invoke_raw("save_bytes", &bytes, &[("x-file-name", &format!("{name}.png"))]).await;
                    if let Err(e) = saved {
                        ed.notify(false, e.to_string());
                    }
                }
                Err(_) => ed.notify(false, "Could not export the image."),
            }
        });
    }

    // ------------------------------------------------------ transformer

    fn frame(&self) -> Option<Frame> {
        if self.editing.get_untracked().is_some() {
            return None;
        }
        let ids = self.selected.get_untracked();
        let sel: Vec<Layer> = self.with_canvas_untracked(|c| {
            c.layers.iter().filter(|l| ids.contains(&l.base.id) && l.base.visible && (ids.len() == 1 || !l.base.locked)).cloned().collect()
        });
        match sel.as_slice() {
            [] => None,
            [l] if ids.len() == 1 => {
                if l.line().is_some() {
                    return None;
                }
                Some(Frame::Single { m: node_mat(l), r: self_rect(l), layer: l.clone() })
            }
            many => Some(Frame::Multi { ids: many.iter().map(|l| l.base.id.clone()).collect(), b: arrange::bounds_of(&many.iter().map(client_rect).collect::<Vec<_>>()) }),
        }
    }
}

fn anchors_for(f: &Frame) -> Vec<Anchor> {
    match f {
        Frame::Multi { .. } => ALL_ANCHORS[..4].to_vec(),
        Frame::Single { layer, .. } if layer.base.locked => Vec::new(),
        Frame::Single { layer, .. } => match layer.shape {
            Shape::Image(_) => ALL_ANCHORS[..4].to_vec(),
            Shape::Text(_) => vec![Anchor::TopLeft, Anchor::TopRight, Anchor::BottomRight, Anchor::BottomLeft, Anchor::RightMid, Anchor::LeftMid],
            _ => ALL_ANCHORS.to_vec(),
        },
    }
}

/// An anchor's spot on the page.
fn anchor_point(f: &Frame, a: Anchor) -> (f64, f64) {
    let (fx, fy) = a.at();
    match f {
        Frame::Single { m, r, .. } => m.apply(r.x + r.width * fx, r.y + r.height * fy),
        Frame::Multi { b, .. } => (b.x + b.width * fx, b.y + b.height * fy),
    }
}

fn frame_center(f: &Frame) -> (f64, f64) {
    match f {
        Frame::Single { m, r, .. } => m.apply(r.x + r.width / 2.0, r.y + r.height / 2.0),
        Frame::Multi { b, .. } => (b.x + b.width / 2.0, b.y + b.height / 2.0),
    }
}

/// The rotate handle: 28 screen px beyond the top edge.
fn rotater(f: &Frame, zoom: f64) -> (f64, f64) {
    let top = anchor_point(f, Anchor::TopMid);
    let c = frame_center(f);
    let (dx, dy) = (top.0 - c.0, top.1 - c.1);
    let len = dx.hypot(dy);
    let (ux, uy) = if len < 1e-9 { (0.0, -1.0) } else { (dx / len, dy / len) };
    (top.0 + ux * 28.0 / zoom, top.1 + uy * 28.0 / zoom)
}

fn rotation_of(f: &Frame) -> f64 {
    match f {
        Frame::Single { layer, .. } => layer.base.rotation,
        Frame::Multi { .. } => 0.0,
    }
}

/// Konva's rotation snaps: every 45° within 4°.
fn snap_rotation(deg: f64) -> f64 {
    let norm = deg.rem_euclid(360.0);
    for s in (0..=8).map(|i| f64::from(i) * 45.0) {
        if (norm - s).abs() < 4.0 {
            return deg - norm + s;
        }
    }
    deg
}

/// Mutates the layers for a resize from `anchor` towards page point `p`.
fn apply_resize(anchor: Anchor, frame: &Frame, origin: &[Layer], p: (f64, f64), zoom: f64, layers: &mut [Layer]) {
    match frame {
        Frame::Single { layer, m, r } => {
            let Some(o) = origin.iter().find(|l| l.id() == layer.id()) else { return };
            let b = &o.base;
            let (ax, ay) = anchor.at();
            let (fx0, fy0) = anchor.opposite().at();
            let pl = (r.x + r.width * fx0, r.y + r.height * fy0);
            let al = (r.x + r.width * ax, r.y + r.height * ay);
            let fixed = m.apply(pl.0, pl.1);
            let rot = b.rotation.to_radians();
            // Pointer relative to the fixed point, in the layer's unscaled local frame.
            let (vx, vy) = (p.0 - fixed.0, p.1 - fixed.1);
            let (lx, ly) = ((vx * rot.cos() + vy * rot.sin()) / b.scale_x, (-vx * rot.sin() + vy * rot.cos()) / b.scale_y);
            let a = (al.0 - pl.0, al.1 - pl.1);
            let text_side = matches!(o.shape, Shape::Text(_)) && matches!(anchor, Anchor::LeftMid | Anchor::RightMid);
            let keep = anchor.corner() && matches!(o.shape, Shape::Image(_) | Shape::Text(_));
            let (mut fx, mut fy) = (1.0, 1.0);
            if keep {
                let f = (lx * a.0 + ly * a.1) / (a.0 * a.0 + a.1 * a.1);
                fx = f;
                fy = f;
            } else {
                if a.0.abs() > 1e-9 {
                    fx = lx / a.0;
                }
                if a.1.abs() > 1e-9 {
                    fy = ly / a.1;
                }
            }
            // No flipping through the handles, and nothing smaller than 8 screen px.
            let min = 8.0 / zoom;
            if fx <= 0.0 || fy <= 0.0 || (r.width * b.scale_x * fx).abs() < min || (r.height * b.scale_y * fy).abs() < min {
                return;
            }
            let Some(l) = layers.iter_mut().find(|l| l.id() == layer.id()) else { return };
            let (ox, oy) = render::offset(o);
            if text_side {
                // Side handles change a text box's width (re-wrapping); corners scale it.
                if let Shape::Text(t) = &mut l.shape
                    && let Shape::Text(t0) = &o.shape
                {
                    t.width = (t0.width * b.scale_x * fx / b.scale_y).max(30.0);
                    l.base.scale_x = b.scale_y;
                    // Keep the opposite side's top corner where it was.
                    let fixed_local = if anchor == Anchor::LeftMid { (t0.width, 0.0) } else { (0.0, 0.0) };
                    let fixed_page = m.apply(fixed_local.0, fixed_local.1);
                    let new_local = if anchor == Anchor::LeftMid { (t.width, 0.0) } else { (0.0, 0.0) };
                    let (sx, sy) = (l.base.scale_x, l.base.scale_y);
                    let (dx, dy) = (sx * (new_local.0 - ox), sy * (new_local.1 - oy));
                    l.base.x = fixed_page.0 - (dx * rot.cos() - dy * rot.sin());
                    l.base.y = fixed_page.1 - (dx * rot.sin() + dy * rot.cos());
                }
                return;
            }
            l.base.scale_x = b.scale_x * fx;
            l.base.scale_y = b.scale_y * fy;
            let (dx, dy) = (l.base.scale_x * (pl.0 - ox), l.base.scale_y * (pl.1 - oy));
            l.base.x = fixed.0 - (dx * rot.cos() - dy * rot.sin());
            l.base.y = fixed.1 - (dx * rot.sin() + dy * rot.cos());
        }
        Frame::Multi { ids, b } => {
            let fixed = anchor_point(frame, anchor.opposite());
            let a = anchor_point(frame, anchor);
            let (ax, ay) = (a.0 - fixed.0, a.1 - fixed.1);
            let f = ((p.0 - fixed.0) * ax + (p.1 - fixed.1) * ay) / (ax * ax + ay * ay);
            if f <= 0.0 || b.width * f < 8.0 / zoom || b.height * f < 8.0 / zoom {
                return;
            }
            for l in layers.iter_mut().filter(|l| ids.contains(&l.base.id)) {
                if let Some(o) = origin.iter().find(|x| x.id() == l.id()) {
                    l.base.x = fixed.0 + (o.base.x - fixed.0) * f;
                    l.base.y = fixed.1 + (o.base.y - fixed.1) * f;
                    l.base.scale_x = o.base.scale_x * f;
                    l.base.scale_y = o.base.scale_y * f;
                }
            }
        }
    }
}

/// Mutates the layers for a rotation towards page point `p` (about the box centre).
fn apply_rotate(frame: &Frame, origin: &[Layer], p: (f64, f64), layers: &mut [Layer]) {
    let c = frame_center(frame);
    let angle = snap_rotation((p.1 - c.1).atan2(p.0 - c.0).to_degrees() + 90.0);
    let delta = (angle - rotation_of(frame)).to_radians();
    let ids: Vec<&str> = match frame {
        Frame::Single { layer, .. } => vec![layer.id()],
        Frame::Multi { ids, .. } => ids.iter().map(String::as_str).collect(),
    };
    for l in layers.iter_mut().filter(|l| ids.contains(&l.id())) {
        if let Some(o) = origin.iter().find(|x| x.id() == l.id()) {
            let (dx, dy) = (o.base.x - c.0, o.base.y - c.1);
            l.base.x = c.0 + dx * delta.cos() - dy * delta.sin();
            l.base.y = c.1 + dx * delta.sin() + dy * delta.cos();
            l.base.rotation = o.base.rotation + delta.to_degrees();
        }
    }
}

fn guide_for(snapped: Option<Snapped>, origin: (f64, f64)) -> (Vec<f64>, Vec<f64>) {
    match snapped {
        Some(Snapped::Horizontal) => (vec![], vec![origin.1]),
        Some(Snapped::Vertical) => (vec![origin.0], vec![]),
        _ => (vec![], vec![]),
    }
}

fn line_points(l: &Layer) -> Vec<f64> {
    l.line().map(|x| x.points.clone()).unwrap_or_default()
}

/// The page with its layers, and everything drawn over it.
fn draw(ed: Ed, el: &web_sys::HtmlCanvasElement) {
    let (bw, bh) = ed.board.get();
    if bw <= 0.0 {
        return;
    }
    let dpr = window().device_pixel_ratio();
    let (pw, ph) = ((bw * dpr).round() as u32, (bh * dpr).round() as u32);
    if el.width() != pw || el.height() != ph {
        el.set_width(pw);
        el.set_height(ph);
    }
    let Some(ctx) = el.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return };
    let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    ctx.clear_rect(0.0, 0.0, bw, bh);
    let (ox, oy, z) = ed.origin();
    let editing = ed.editing.get();
    let selected = ed.selected.get();
    images_tick().track();
    ed.with_canvas(|c| {
        let (w, h) = (c.width, c.height);
        // Page shadow.
        ctx.save();
        ctx.set_shadow_color("rgba(44, 51, 47, 0.18)");
        ctx.set_shadow_blur(30.0);
        ctx.set_fill_style_str("#ffffff");
        ctx.fill_rect(ox, oy, w * z, h * z);
        ctx.restore();
        // The page, clipped.
        ctx.save();
        let _ = ctx.translate(ox, oy);
        let _ = ctx.scale(z, z);
        ctx.begin_path();
        ctx.rect(0.0, 0.0, w, h);
        ctx.clip();
        render::draw_page(&ctx, c, &|l| editing.as_deref() == Some(l.id()));
        ctx.restore();

        // Overlays, in page coordinates, outside the clip.
        ctx.save();
        let _ = ctx.translate(ox, oy);
        let _ = ctx.scale(z, z);
        let px = 1.0 / z;
        let dash = |a: f64, b: f64| {
            let arr = js_sys::Array::of2(&(a * px).into(), &(b * px).into());
            let _ = ctx.set_line_dash(&arr);
        };
        let no_dash = || {
            let _ = ctx.set_line_dash(&js_sys::Array::new());
        };
        let (gv, gh) = ed.guides.get();
        if let Some(r) = ed.hover.get().filter(|_| gv.is_empty() && gh.is_empty() && ed.marquee.with(Option::is_none)) {
            ctx.set_stroke_style_str(ACCENT);
            ctx.set_line_width(1.5 * px);
            dash(4.0, 3.0);
            ctx.stroke_rect(r.x, r.y, r.width, r.height);
            no_dash();
        }
        ctx.set_stroke_style_str(GUIDE);
        ctx.set_line_width(px);
        for x in gv {
            ctx.begin_path();
            ctx.move_to(x, -40.0 * px);
            ctx.line_to(x, h + 40.0 * px);
            ctx.stroke();
        }
        for y in gh {
            ctx.begin_path();
            ctx.move_to(-40.0 * px, y);
            ctx.line_to(w + 40.0 * px, y);
            ctx.stroke();
        }
        // Drawing preview, MS Paint style.
        if let Some((s, e, tool)) = ed.drawing.get() {
            ctx.begin_path();
            ctx.move_to(s.0, s.1);
            if tool == Tool::Curve {
                let (cx, cy) = ((s.0 + e.0) / 2.0 - (e.1 - s.1) * 0.2, (s.1 + e.1) / 2.0 + (e.0 - s.0) * 0.2);
                ctx.quadratic_curve_to(cx, cy, e.0, e.1);
            } else {
                ctx.line_to(e.0, e.1);
            }
            ctx.set_stroke_style_str("#2c332f");
            ctx.set_line_width(4.0 * px);
            ctx.set_line_cap("round");
            ctx.stroke();
        }
        // Line and curve handles.
        let one = if selected.len() == 1 { c.layers.iter().find(|l| l.base.id == selected[0]) } else { None };
        if let Some(l) = one.filter(|l| !l.base.locked && l.line().is_some()) {
            let p = line_points(l);
            let at = |i: usize| (l.base.x + p.get(i * 2).copied().unwrap_or(0.0), l.base.y + p.get(i * 2 + 1).copied().unwrap_or(0.0));
            let handle = |pt: (f64, f64), bend: bool| {
                ctx.begin_path();
                let _ = ctx.arc(pt.0, pt.1, if bend { 9.5 } else { 8.0 } * px, 0.0, std::f64::consts::TAU);
                ctx.set_fill_style_str(if bend { ACCENT } else { "#ffffff" });
                ctx.fill();
                ctx.set_line_width(2.5 * px);
                ctx.set_stroke_style_str(if bend { "#ffffff" } else { ACCENT });
                ctx.stroke();
            };
            if l.kind() == "curve" {
                ctx.begin_path();
                ctx.move_to(at(0).0, at(0).1);
                ctx.line_to(at(1).0, at(1).1);
                ctx.line_to(at(2).0, at(2).1);
                ctx.set_stroke_style_str(ACCENT);
                ctx.set_line_width(1.5 * px);
                dash(4.0, 4.0);
                ctx.stroke();
                no_dash();
                handle(at(0), false);
                handle(at(1), true);
                handle(at(2), false);
            } else {
                handle(at(0), false);
                handle(at(1), false);
            }
        }
        ctx.restore();
    });
    // The transformer, in board coordinates so its handles stay the same size at any zoom.
    if let Some(f) = ed.frame() {
        let to_board = |p: (f64, f64)| (ox + p.0 * z, oy + p.1 * z);
        let locked = matches!(&f, Frame::Single { layer, .. } if layer.base.locked);
        let corners: Vec<(f64, f64)> = [Anchor::TopLeft, Anchor::TopRight, Anchor::BottomRight, Anchor::BottomLeft].iter().map(|a| to_board(anchor_point(&f, *a))).collect();
        ctx.begin_path();
        for (i, (x, y)) in corners.iter().enumerate() {
            if i == 0 {
                ctx.move_to(*x, *y);
            } else {
                ctx.line_to(*x, *y);
            }
        }
        ctx.close_path();
        ctx.set_stroke_style_str(if locked { LOCKED } else { ACCENT });
        ctx.set_line_width(1.5);
        ctx.stroke();
        if !locked {
            let (rx, ry) = to_board(rotater(&f, z));
            let top = to_board(anchor_point(&f, Anchor::TopMid));
            ctx.begin_path();
            ctx.move_to(top.0, top.1);
            ctx.line_to(rx, ry);
            ctx.stroke();
            for (x, y, r) in std::iter::once((rx, ry, 6.0)).chain(anchors_for(&f).into_iter().map(|a| {
                let (x, y) = to_board(anchor_point(&f, a));
                (x, y, 5.0)
            })) {
                ctx.begin_path();
                let _ = ctx.arc(x, y, r, 0.0, std::f64::consts::TAU);
                ctx.set_fill_style_str(HANDLE_FILL);
                ctx.fill();
                ctx.set_line_width(1.5);
                ctx.set_stroke_style_str(ACCENT);
                ctx.stroke();
            }
        }
    }
}

#[component]
pub fn Studio(config: StudioConfig) -> impl IntoView {
    let initial = config.initial.clone();
    let ed = Ed {
        hist: RwSignal::new(History::new(initial.clone())),
        saved: RwSignal::new(initial),
        selected: RwSignal::new(Vec::new()),
        editing: RwSignal::new(None),
        menu: RwSignal::new(None),
        clipboard: StoredValue::new(Vec::new()),
        tool: RwSignal::new(Tool::Select),
        snap_right: RwSignal::new(true),
        view: RwSignal::new(View::default()),
        board: RwSignal::new((0.0, 0.0)),
        guides: RwSignal::new((Vec::new(), Vec::new())),
        hover: RwSignal::new(None),
        marquee: RwSignal::new(None),
        drawing: RwSignal::new(None),
        gesture: StoredValue::new(None),
        toast: RwSignal::new(None),
        saving: RwSignal::new(false),
        swap_id: RwSignal::new(None),
        uploads: RwSignal::new(Vec::new()),
        uploading: RwSignal::new(false),
        removing: RwSignal::new(false),
        space: RwSignal::new(false),
        tab: RwSignal::new(Some(Tab::Products)),
        config: StoredValue::new_local(config),
        board_el: NodeRef::new(),
    };
    let canvas_el = NodeRef::<html::Canvas>::new();
    super::ensure_font_stylesheet();

    // Web fonts load on demand; redraw once each face in use has arrived.
    let loaded_faces = StoredValue::new(Vec::<String>::new());
    Effect::new(move |_| {
        let c = ed.canvas();
        let faces: Vec<String> = c.layers.iter().filter_map(|l| match &l.shape {
            Shape::Text(t) => Some(format!("{}|{}", t.font_style.css(), t.font_family)),
            _ => None,
        }).collect();
        if faces.iter().all(|f| loaded_faces.with_value(|v| v.contains(f))) {
            return;
        }
        loaded_faces.update_value(|v| v.extend(faces));
        spawn_local(async move {
            super::load_fonts(&c).await;
            super::images_tick().update(|n| *n += 1);
        });
    });

    // Board size.
    Effect::new(move |_| {
        let Some(el) = ed.board_el.get() else { return };
        let measure = {
            let el = el.clone();
            move || ed.board.set((f64::from(el.client_width()), f64::from(el.client_height())))
        };
        measure();
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(measure);
        if let Ok(ro) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
            ro.observe(&el);
            std::mem::forget(ro);
        }
        cb.forget();
    });

    // Redraw whenever anything drawn changes.
    Effect::new(move |_| {
        if let Some(el) = canvas_el.get() {
            draw(ed, &el);
        }
    });

    let pointer_down = move |e: web_sys::PointerEvent| {
        ed.menu.set(None);
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let target = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        if let Some(t) = &target {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        if ed.space.get_untracked() || e.button() == 1 {
            e.prevent_default();
            ed.gesture.set_value(Some(Gesture::Pan { last: (cx, cy) }));
            return;
        }
        if e.button() != 0 {
            return;
        }
        let p = ed.page_point(cx, cy);
        let tool = ed.tool.get_untracked();
        if tool != Tool::Select {
            ed.drawing.set(Some((p, p, tool)));
            ed.gesture.set_value(Some(Gesture::Draw));
            return;
        }
        let z = ed.zoom();
        let near = |a: (f64, f64), r: f64| (a.0 - p.0).hypot(a.1 - p.1) * z <= r;
        let group = format!("gesture:{}", now_ms());
        // Transformer handles first.
        if let Some(f) = ed.frame() {
            let locked = matches!(&f, Frame::Single { layer, .. } if layer.base.locked);
            if !locked {
                let origin = ed.with_canvas_untracked(|c| c.layers.clone());
                if near(rotater(&f, z), 10.0) {
                    ed.gesture.set_value(Some(Gesture::Rotate { origin, frame: f, group }));
                    return;
                }
                if let Some(a) = anchors_for(&f).into_iter().find(|a| near(anchor_point(&f, *a), 9.0)) {
                    ed.gesture.set_value(Some(Gesture::Resize { anchor: a, origin, frame: f, group }));
                    return;
                }
            }
        }
        // Line and curve handles.
        let sel = ed.selected.get_untracked();
        if sel.len() == 1
            && let Some(l) = ed.with_canvas_untracked(|c| c.layers.iter().find(|l| l.base.id == sel[0] && !l.base.locked && l.line().is_some()).cloned())
        {
            let pts = line_points(&l);
            let n = pts.len() / 2;
            for i in 0..n {
                let at = (l.base.x + pts[i * 2], l.base.y + pts[i * 2 + 1]);
                if near(at, 12.0) {
                    ed.gesture.set_value(Some(Gesture::Handle { id: l.base.id.clone(), which: i, group }));
                    return;
                }
            }
        }
        let editing = ed.editing.get_untracked();
        let top = ed.with_canvas_untracked(|c| c.layers.iter().rev().find(|l| l.base.visible && editing.as_deref() != Some(l.id()) && hit(l, p.0, p.1, z)).cloned());
        let additive = e.shift_key() || e.ctrl_key() || e.meta_key();
        match top {
            Some(l) => {
                let id = l.base.id.clone();
                if additive {
                    ed.toggle(&id);
                } else if !sel.contains(&id) {
                    // Pressing on part of a selection keeps it, so the group drags together.
                    ed.select(Some(id.clone()));
                }
                if l.base.locked {
                    return;
                }
                let now_sel = ed.selected.get_untracked();
                let ids: Vec<String> = if now_sel.contains(&id) { now_sel } else { vec![id] };
                let (origin, rects, others) = ed.with_canvas_untracked(|c| {
                    let moving: Vec<Layer> = c.layers.iter().filter(|x| ids.contains(&x.base.id) && !x.base.locked).cloned().collect();
                    let rects = moving.iter().map(client_rect).collect();
                    let others = c.layers.iter().filter(|x| !ids.contains(&x.base.id) && x.base.visible).map(client_rect).collect();
                    (moving, rects, others)
                });
                ed.hover.set(None);
                ed.gesture.set_value(Some(Gesture::Drag { ids: origin.iter().map(|x| x.base.id.clone()).collect(), start: p, origin, rects, others, group }));
            }
            None => {
                if !additive {
                    ed.select(None);
                }
                let keep = if additive { ed.selected.get_untracked() } else { Vec::new() };
                ed.gesture.set_value(Some(Gesture::Marquee { start: ed.board_point(cx, cy), keep }));
            }
        }
    };

    let pointer_move = move |e: web_sys::PointerEvent| {
        let (cx, cy) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let p = ed.page_point(cx, cy);
        let z = ed.zoom();
        let Some(g) = ed.gesture.get_value() else {
            // Hover outline.
            if ed.tool.get_untracked() != Tool::Select || ed.space.get_untracked() {
                return ed.hover.set(None);
            }
            let sel = ed.selected.get_untracked();
            let r = ed.with_canvas_untracked(|c| c.layers.iter().rev().find(|l| l.base.visible && hit(l, p.0, p.1, z)).filter(|l| !sel.contains(&l.base.id)).map(client_rect));
            if ed.hover.get_untracked() != r {
                ed.hover.set(r);
            }
            return;
        };
        match g {
            Gesture::Pan { last } => {
                ed.gesture.set_value(Some(Gesture::Pan { last: (cx, cy) }));
                let z = ed.zoom();
                ed.view.update(|v| {
                    v.zoom = Some(v.zoom.unwrap_or(z));
                    v.pan_x += cx - last.0;
                    v.pan_y += cy - last.1;
                });
            }
            Gesture::Draw => {
                if let Some((s, _, tool)) = ed.drawing.get_untracked() {
                    let (x, y, snapped) = arrange::snap_to_right_angle(s, p, ed.snap_right.get_untracked(), e.shift_key());
                    ed.drawing.set(Some((s, (x, y), tool)));
                    ed.guides.set(guide_for(snapped, s));
                }
            }
            Gesture::Drag { ids, start, origin, rects, others, group } => {
                let (mut dx, mut dy) = (p.0 - start.0, p.1 - start.1);
                if e.alt_key() {
                    ed.guides.set((vec![], vec![]));
                } else {
                    let moved: Vec<Rect> = rects.iter().map(|r| Rect { x: r.x + dx, y: r.y + dy, ..*r }).collect();
                    let (w, h) = ed.with_canvas_untracked(|c| (c.width, c.height));
                    let s = arrange::snap(arrange::bounds_of(&moved), &others, w, h, 6.0 / z);
                    dx += s.dx;
                    dy += s.dy;
                    if ed.guides.with_untracked(|g| g.0 != s.vertical || g.1 != s.horizontal) {
                        ed.guides.set((s.vertical, s.horizontal));
                    }
                }
                ed.change(
                    |c| {
                        for l in c.layers.iter_mut().filter(|l| ids.contains(&l.base.id)) {
                            if let Some(o) = origin.iter().find(|o| o.id() == l.id()) {
                                l.base.x = o.base.x + dx;
                                l.base.y = o.base.y + dy;
                            }
                        }
                    },
                    Some(&group),
                );
            }
            Gesture::Resize { anchor, origin, frame, group } => {
                ed.change(|c| apply_resize(anchor, &frame, &origin, p, z, &mut c.layers), Some(&group));
            }
            Gesture::Rotate { origin, frame, group } => {
                ed.change(|c| apply_rotate(&frame, &origin, p, &mut c.layers), Some(&group));
            }
            Gesture::Handle { id, which, group } => {
                let Some(l) = ed.with_canvas_untracked(|c| c.layers.iter().find(|l| l.base.id == id).cloned()) else { return };
                let pts = line_points(&l);
                let at = |i: usize| (l.base.x + pts.get(i * 2).copied().unwrap_or(0.0), l.base.y + pts.get(i * 2 + 1).copied().unwrap_or(0.0));
                let curve = l.kind() == "curve";
                let end_i = if curve { 2 } else { 1 };
                let snapped = |origin: (f64, f64)| arrange::snap_to_right_angle(origin, p, ed.snap_right.get_untracked(), e.shift_key());
                let new_points: Option<(f64, f64, Vec<f64>, Option<f64>)> = match (which, curve) {
                    (0, _) => {
                        // Start: snap against the end, move the layer origin.
                        let end = at(end_i);
                        let (sx, sy, s) = snapped(end);
                        let (x, y) = (sx.round(), sy.round());
                        ed.guides.set(guide_for(s, end));
                        let (ddx, ddy) = (x - l.base.x, y - l.base.y);
                        let mut np = vec![0.0, 0.0];
                        for i in 1..pts.len() / 2 {
                            np.push(pts[i * 2] - ddx);
                            np.push(pts[i * 2 + 1] - ddy);
                        }
                        Some((x, y, np, None))
                    }
                    (1, true) => {
                        // The bend: its offset from the chord sets the curvature.
                        let (ncx, ncy) = ((p.0 - l.base.x).round(), (p.1 - l.base.y).round());
                        let (ex, ey) = (pts.get(4).copied().unwrap_or(300.0), pts.get(5).copied().unwrap_or(0.0));
                        let len = ex.hypot(ey).max(1.0);
                        let proj = (ncx - ex / 2.0) * (-ey / len) + (ncy - ey / 2.0) * (ex / len);
                        let curvature = (proj * 2.0 / len * 100.0).round() / 100.0;
                        Some((l.base.x, l.base.y, vec![0.0, 0.0, ncx, ncy, ex, ey], Some(curvature)))
                    }
                    _ => {
                        // End: snap against the start, stored relative to it.
                        let start = (l.base.x, l.base.y);
                        let (sx, sy, s) = snapped(start);
                        ed.guides.set(guide_for(s, start));
                        let (rx, ry) = ((sx - start.0).round(), (sy - start.1).round());
                        let mut np = pts.clone();
                        let n = np.len();
                        if n >= 2 {
                            np[n - 2] = rx;
                            np[n - 1] = ry;
                        }
                        Some((l.base.x, l.base.y, np, None))
                    }
                };
                if let Some((x, y, np, curvature)) = new_points {
                    ed.patch(
                        &id,
                        |l| {
                            l.base.x = x;
                            l.base.y = y;
                            if let Some(line) = l.line_mut() {
                                line.points = np;
                            }
                            if let (Some(k), Shape::Curve(c)) = (curvature, &mut l.shape) {
                                c.curvature = k;
                            }
                        },
                        Some(&group),
                    );
                }
            }
            Gesture::Marquee { start, .. } => {
                let (bx, by) = ed.board_point(cx, cy);
                ed.marquee.set(Some(Rect { x: start.0.min(bx), y: start.1.min(by), width: (bx - start.0).abs(), height: (by - start.1).abs() }));
            }
        }
    };

    let pointer_up = move |_: web_sys::PointerEvent| {
        let g = ed.gesture.get_value();
        ed.gesture.set_value(None);
        ed.guides.set((vec![], vec![]));
        match g {
            Some(Gesture::Marquee { keep, .. }) => {
                let area = ed.marquee.get_untracked();
                ed.marquee.set(None);
                let Some(a) = area.filter(|a| a.width >= 4.0 || a.height >= 4.0) else { return };
                let (ox, oy, z) = ed.origin();
                // Board box → page box.
                let pa = Rect { x: (a.x - ox) / z, y: (a.y - oy) / z, width: a.width / z, height: a.height / z };
                let hits: Vec<String> = ed.with_canvas_untracked(|c| c.layers.iter().filter(|l| l.base.visible && !l.base.locked && arrange::intersects(client_rect(l), pa)).map(|l| l.base.id.clone()).collect());
                let mut ids = keep;
                for h in hits {
                    if !ids.contains(&h) {
                        ids.push(h);
                    }
                }
                ed.select_many(ids);
            }
            Some(Gesture::Draw) => {
                let Some((s, e, tool)) = ed.drawing.get_untracked() else { return };
                ed.drawing.set(None);
                ed.tool.set(Tool::Select);
                let (dx, dy) = (e.0 - s.0, e.1 - s.1);
                let dist = dx.hypot(dy);
                if dist < 8.0 {
                    return;
                }
                let line = |points: Vec<f64>| Line { points, stroke: "#2c332f".into(), stroke_width: 4.0, dash: None, line_cap: LineCap::Round };
                let shape = if tool == Tool::Line {
                    Shape::Line(line(vec![0.0, 0.0, dx.round(), dy.round()]))
                } else {
                    let off = dist * 0.4 / 2.0;
                    let (cx, cy) = (dx / 2.0 + (-dy / dist) * off, dy / 2.0 + (dx / dist) * off);
                    Shape::Curve(Curve { line: line(vec![0.0, 0.0, cx.round(), cy.round(), dx.round(), dy.round()]), curvature: 0.4 })
                };
                ed.add(Layer { base: Base::at(uuid(), s.0.round(), s.1.round()), shape });
            }
            _ => {}
        }
    };

    let dbl_click = move |e: web_sys::MouseEvent| {
        let p = ed.page_point(f64::from(e.client_x()), f64::from(e.client_y()));
        let z = ed.zoom();
        let top = ed.with_canvas_untracked(|c| c.layers.iter().rev().find(|l| l.base.visible && hit(l, p.0, p.1, z)).cloned());
        if let Some(l) = top.filter(|l| l.kind() == "text" && !l.base.locked) {
            ed.select(Some(l.base.id.clone()));
            ed.editing.set(Some(l.base.id));
        }
    };

    let context_menu = move |e: web_sys::MouseEvent| {
        e.prevent_default();
        let p = ed.page_point(f64::from(e.client_x()), f64::from(e.client_y()));
        let z = ed.zoom();
        let top = ed.with_canvas_untracked(|c| c.layers.iter().rev().find(|l| l.base.visible && hit(l, p.0, p.1, z)).map(|l| l.base.id.clone()));
        match top {
            Some(id) if !ed.selected.get_untracked().contains(&id) => ed.select(Some(id)),
            Some(_) => {}
            None => ed.select(None),
        }
        ed.menu.set(Some((f64::from(e.client_x()), f64::from(e.client_y()))));
    };

    let wheel = move |e: web_sys::WheelEvent| {
        e.prevent_default();
        if e.ctrl_key() || e.meta_key() {
            let at = ed.board_point(f64::from(e.client_x()), f64::from(e.client_y()));
            let d = e.delta_y();
            ed.zoom_to(move |z| z * (-d * 0.0015).exp(), Some(at));
        } else {
            let z = ed.zoom();
            let (dx, dy) = (e.delta_x(), e.delta_y());
            ed.view.update(|v| {
                v.zoom = Some(v.zoom.unwrap_or(z));
                v.pan_x -= dx;
                v.pan_y -= dy;
            });
        }
    };

    let drop = move |e: web_sys::DragEvent| {
        let Some(dt) = e.data_transfer() else { return };
        let Ok(raw) = dt.get_data(DRAG_MIME) else { return };
        if raw.is_empty() {
            return;
        }
        e.prevent_default();
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { return };
        let at = Some(ed.page_point(f64::from(e.client_x()), f64::from(e.client_y())));
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or_default().to_string();
        ed.swap_id.set(None);
        match v.get("product").filter(|p| !p.is_null()) {
            Some(p) => {
                let s = |k: &str| p.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
                ed.place_image(PickerProduct { id: s("id"), code: s("code"), name: s("name"), image_url: Some(url.clone()) }, url, at);
            }
            None => ed.add_image(url, v.get("name").and_then(|n| n.as_str()).unwrap_or("Image").to_string(), at),
        }
    };

    // Keyboard shortcuts.
    let navigate = StoredValue::new_local(use_navigate());
    let _keys = window_event_listener(leptos::ev::keydown, move |e| {
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
            .is_some_and(|t| matches!(t.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || t.is_content_editable());
        let m = e.ctrl_key() || e.meta_key();
        let key = e.key().to_lowercase();
        if m && key == "s" {
            e.prevent_default();
            spawn_local(async move {
                ed.save().await;
            });
            return;
        }
        if typing {
            return;
        }
        let sel = ed.selected.get_untracked();
        let has = !sel.is_empty();
        match () {
            _ if e.code() == "Space" => {
                e.prevent_default();
                if !e.repeat() {
                    ed.space.set(true);
                }
            }
            _ if m && key == "z" => {
                e.prevent_default();
                if e.shift_key() { ed.redo() } else { ed.undo() }
            }
            _ if m && key == "y" => {
                e.prevent_default();
                ed.redo();
            }
            _ if m && key == "a" => {
                e.prevent_default();
                ed.select_all();
            }
            _ if m && key == "c" && has => ed.copy(),
            _ if m && key == "v" => {
                e.prevent_default();
                ed.paste();
            }
            _ if m && key == "d" && has => {
                e.prevent_default();
                ed.duplicate(&sel);
            }
            _ if m && (e.code() == "BracketRight" || e.code() == "BracketLeft") && has => {
                e.prevent_default();
                let up = e.code() == "BracketRight";
                ed.move_layers(&sel, match (e.shift_key(), up) {
                    (true, true) => LayerMove::Front,
                    (true, false) => LayerMove::Back,
                    (false, true) => LayerMove::Forward,
                    (false, false) => LayerMove::Backward,
                });
            }
            _ if m && key == "0" => {
                e.prevent_default();
                ed.zoom_fit();
            }
            _ if m && (key == "=" || key == "+") => {
                e.prevent_default();
                ed.zoom_to(|z| z * 1.25, None);
            }
            _ if m && key == "-" => {
                e.prevent_default();
                ed.zoom_to(|z| z / 1.25, None);
            }
            _ if (key == "delete" || key == "backspace") && has => {
                e.prevent_default();
                ed.remove(&sel);
            }
            _ if key.starts_with("arrow") && has => {
                e.prevent_default();
                let step = if e.shift_key() { 10.0 } else { 1.0 };
                let (dx, dy) = match key.as_str() {
                    "arrowleft" => (-step, 0.0),
                    "arrowright" => (step, 0.0),
                    "arrowup" => (0.0, -step),
                    _ => (0.0, step),
                };
                let ids: Vec<String> = ed.with_canvas_untracked(|c| c.layers.iter().filter(|l| sel.contains(&l.base.id) && !l.base.locked).map(|l| l.base.id.clone()).collect());
                let group = format!("nudge:{}", ids.join(","));
                ed.shift(&ids.iter().map(|i| (i.clone(), dx, dy)).collect::<Vec<_>>(), Some(&group));
            }
            _ if key == "enter" => {
                if let Some(l) = ed.selected_one().filter(|l| l.kind() == "text" && !l.base.locked) {
                    e.prevent_default();
                    ed.editing.set(Some(l.base.id));
                }
            }
            _ if key == "escape" => {
                if ed.menu.get_untracked().is_some() {
                    ed.menu.set(None);
                } else if ed.swap_id.get_untracked().is_some() {
                    ed.swap_id.set(None);
                } else {
                    ed.select(None);
                }
            }
            _ => {}
        }
    });
    let _keyup = window_event_listener(leptos::ev::keyup, move |e| {
        if e.code() == "Space" {
            ed.space.set(false);
        }
    });
    let _ = navigate;

    // The editor is full screen; stop the page behind it scrolling.
    if let Some(body) = document().body() {
        let _ = body.style().set_property("overflow", "hidden");
    }
    on_cleanup(|| {
        if let Some(body) = document().body() {
            let _ = body.style().remove_property("overflow");
        }
    });

    let cursor = move || {
        if ed.space.get() {
            "cursor-grab"
        } else if ed.tool.get() != Tool::Select {
            "cursor-crosshair"
        } else {
            ""
        }
    };
    let has_pages = ed.config.with_value(|c| !c.pages.is_empty());

    view! {
        <div class="studio fixed inset-0 z-[60] flex flex-col bg-(--st-bg)">
            <Topbar ed=ed />
            <div class="flex min-h-0 flex-1">
                <Rail ed=ed />
                <Show when=move || ed.tab.get().is_some()><SidePanel ed=ed /></Show>
                <div class="flex min-w-0 flex-1 flex-col">
                    <ContextToolbar ed=ed />
                    <div node_ref=ed.board_el class=move || format!("relative min-h-0 flex-1 overflow-hidden {}", cursor())
                        on:dragover=move |e: web_sys::DragEvent| {
                            if e.data_transfer().is_some_and(|d| d.types().includes(&DRAG_MIME.into(), 0)) {
                                e.prevent_default();
                            }
                        }
                        on:drop=drop
                        on:wheel=wheel>
                        <canvas node_ref=canvas_el class="absolute inset-0 h-full w-full touch-none"
                            on:pointerdown=pointer_down on:pointermove=pointer_move on:pointerup=pointer_up
                            on:dblclick=dbl_click on:contextmenu=context_menu></canvas>
                        <TextEditor ed=ed />
                        <PageResizeHandles ed=ed />
                        {move || ed.marquee.get().map(|m| view! {
                            <div class="pointer-events-none absolute rounded-sm border border-(--st-accent) bg-(--st-accent-soft)"
                                style=format!("left: {}px; top: {}px; width: {}px; height: {}px", m.x, m.y, m.width, m.height)></div>
                        })}
                        <Show when=move || ed.swap_id.get().is_some()>
                            <div class="pointer-events-none absolute top-3 left-1/2 -translate-x-1/2 rounded-full bg-(--st-accent) px-4 py-1.5 text-[12px] font-medium text-(--st-on-accent) shadow-lg">
                                "Choose a new image from the Products panel"
                            </div>
                        </Show>
                        {move || ed.toast.get().map(|(ok, text)| view! {
                            <div role="status" class=format!("absolute bottom-4 left-1/2 max-w-md -translate-x-1/2 rounded-lg px-4 py-2.5 text-[13px] shadow-2xl {}",
                                if ok { "bg-(--st-accent-strong) text-(--st-on-accent)" } else { "bg-red-700 text-white" })>{text}</div>
                        })}
                    </div>
                    {has_pages.then(|| view! { <PageStrip ed=ed /> })}
                    <Footer ed=ed />
                </div>
                <aside class="hidden w-[280px] shrink-0 flex-col border-l border-(--st-line) bg-(--st-panel) lg:flex">
                    <LayersPanel ed=ed />
                    <TransformPanel ed=ed />
                </aside>
            </div>
            <ContextMenu ed=ed />
        </div>
    }
}

/// Inline text editing, over the text it replaces.
#[component]
fn TextEditor(ed: Ed) -> impl IntoView {
    let area = NodeRef::<html::Textarea>::new();
    let layer = Memo::new(move |_| {
        let id = ed.editing.get()?;
        ed.with_canvas(|c| c.layers.iter().find(|l| l.base.id == id).cloned())
    });
    Effect::new(move |_| {
        let _ = layer.get();
        let _ = ed.view.get();
        if let Some(el) = area.get() {
            let st = web_sys::HtmlElement::style(&el);
            let _ = st.set_property("height", "0px");
            let _ = st.set_property("height", &format!("{}px", el.scroll_height()));
        }
    });
    // Focus and select once, when editing starts.
    Effect::new(move |prev: Option<Option<String>>| {
        let id = ed.editing.get();
        if id.is_some() && prev.flatten() != id {
            request_animation_frame(move || {
                if let Some(el) = area.get_untracked() {
                    let _ = el.focus();
                    el.select();
                }
            });
        }
        id
    });
    view! {
        {move || layer.get().and_then(|l| match &l.shape {
            Shape::Text(t) => Some((l.clone(), t.clone())),
            _ => None,
        }).map(|(l, t)| {
            let (ox, oy, z) = ed.origin();
            let b = &l.base;
            let id = b.id.clone();
            let style = format!(
                "left: {}px; top: {}px; width: {}px; font-size: {}px; font-family: \"{}\"; font-weight: {}; font-style: {}; text-align: {}; line-height: {}; letter-spacing: {}px; color: {}; opacity: {}; transform: rotate({}deg); transform-origin: top left; white-space: pre-wrap; overflow-wrap: break-word",
                ox + b.x * z, oy + b.y * z, t.width * b.scale_x.abs() * z, t.font_size * b.scale_y.abs() * z, t.font_family,
                if t.font_style.bold() { 700 } else { 400 }, if t.font_style.italic() { "italic" } else { "normal" },
                match t.align { lattice_core::canvas::Align::Left => "left", lattice_core::canvas::Align::Center => "center", lattice_core::canvas::Align::Right => "right" },
                t.line_height, t.letter_spacing * b.scale_x.abs() * z, t.fill.main_color(), b.opacity, b.rotation
            );
            view! {
                <textarea node_ref=area spellcheck="false" style=style prop:value=t.text.clone()
                    class="absolute resize-none overflow-hidden border-0 bg-transparent p-0 outline-2 outline-(--st-accent)"
                    on:input=move |e| {
                        let v = event_target_value(&e);
                        ed.tweak(&id, "text", |l| if let Shape::Text(t) = &mut l.shape { t.text = v });
                    }
                    on:blur=move |_| ed.editing.set(None)
                    on:keydown={move |e| if e.key() == "Escape" {
                        if let Some(el) = area.get_untracked() {
                            let _ = el.blur();
                        }
                    }}></textarea>
            }
        })}
    }
}

/// Handles on the page's right and bottom edges; dragging resizes the page.
#[component]
fn PageResizeHandles(ed: Ed) -> impl IntoView {
    let resizing = RwSignal::new(false);
    let start = move |e: web_sys::PointerEvent, edge: &'static str| {
        e.prevent_default();
        e.stop_propagation();
        let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let _ = el.set_pointer_capture(e.pointer_id());
        let z = ed.zoom();
        let (w0, h0) = ed.with_canvas_untracked(|c| (c.width, c.height));
        let v0 = ed.view.get_untracked();
        let from = (f64::from(e.client_x()), f64::from(e.client_y()));
        resizing.set(true);
        let size = |v: f64| v.clamp(100.0, 8000.0).round();
        let mv = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
            let w = if edge == "s" { w0 } else { size(w0 + (f64::from(ev.client_x()) - from.0) / z) };
            let h = if edge == "e" {
                h0
            } else if edge == "se" && ev.shift_key() {
                size(w * h0 / w0)
            } else {
                size(h0 + (f64::from(ev.client_y()) - from.1) / z)
            };
            ed.change(
                |c| {
                    c.width = w;
                    c.height = h;
                },
                Some("page-size"),
            );
            ed.view.set(View { zoom: Some(z), pan_x: v0.pan_x + (w - w0) * z / 2.0, pan_y: v0.pan_y + (h - h0) * z / 2.0 });
        });
        let el2 = el.clone();
        let mv_fn: js_sys::Function = mv.as_ref().unchecked_ref::<js_sys::Function>().clone();
        let up = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            let _ = el2.remove_event_listener_with_callback("pointermove", &mv_fn);
            resizing.set(false);
        });
        let _ = el.add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref());
        let _ = el.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
        mv.forget();
        up.forget();
    };
    view! {
        <Show when={move || ed.editing.get().is_none() && ed.board.get().0 > 0.0}>
            {move || {
                let (ox, oy, z) = ed.origin();
                let (w, h) = ed.with_canvas(|c| (c.width, c.height));
                [("e", "h-9 w-2.5 cursor-ew-resize rounded-full", ox + w * z, oy + h * z / 2.0),
                 ("s", "h-2.5 w-9 cursor-ns-resize rounded-full", ox + w * z / 2.0, oy + h * z),
                 ("se", "h-4 w-4 cursor-nwse-resize rounded-full", ox + w * z, oy + h * z)]
                    .into_iter()
                    .map(|(edge, class, x, y)| view! {
                        <div title="Drag to resize the page (Shift on the corner keeps proportions)"
                            class=format!("absolute z-10 -translate-x-1/2 -translate-y-1/2 border-2 border-(--st-accent) bg-(--st-panel) shadow-sm hover:bg-(--st-gold) {class}")
                            style=format!("left: {x}px; top: {y}px")
                            on:pointerdown=move |e| start(e, edge)></div>
                    })
                    .collect_view()
            }}
            <Show when=move || resizing.get()>
                {move || {
                    let (ox, oy, z) = ed.origin();
                    let (w, h) = ed.with_canvas(|c| (c.width, c.height));
                    view! {
                        <div class="pointer-events-none absolute z-10 rounded-md bg-(--st-accent-strong) px-2 py-1 text-[12px] text-(--st-on-accent) tabular-nums shadow"
                            style=format!("left: {}px; top: {}px", ox + w * z + 14.0, oy + h * z + 14.0)>{format!("{w} × {h} px")}</div>
                    }
                }}
            </Show>
        </Show>
    }
}
