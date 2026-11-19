//! Draws a canvas document with Canvas 2D exactly as Konva 10.5 drew it in
//! the old app (Konva is itself a Canvas 2D library): the same transform
//! order, shape paths, text layout and wrapping, gradients and strokes. The
//! editor, thumbnails, the hamper PNG and the PowerPoint pictures all go
//! through here, so they always agree.

use super::{image, load_fonts, load_image};
use lattice_core::api::ApiError;
use lattice_core::canvas::arrange::Box as Rect;
use lattice_core::canvas::{Canvas, Decoration, Fill, Layer, Shape, Text, background_cover, contain_box, gradient_line};
use std::cell::RefCell;
use std::f64::consts::PI;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d as Ctx, HtmlCanvasElement};

/// A 2D affine transform, as canvas `setTransform(a, b, c, d, e, f)` takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Mat {
    pub const ID: Self = Self { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    pub fn mul(&self, o: &Self) -> Self {
        Self {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }

    pub fn invert(&self) -> Self {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 {
            return Self::ID;
        }
        Self {
            a: self.d / det,
            b: -self.b / det,
            c: -self.c / det,
            d: self.a / det,
            e: (self.c * self.f - self.d * self.e) / det,
            f: (self.b * self.e - self.a * self.f) / det,
        }
    }
}

/// Konva's node transform: translate(x, y) · rotate · scale · translate(-offset).
pub fn node_mat(l: &Layer) -> Mat {
    let b = &l.base;
    let r = b.rotation * PI / 180.0;
    let (cos, sin) = (r.cos(), r.sin());
    let (ox, oy) = offset(l);
    let m = Mat { a: cos * b.scale_x, b: sin * b.scale_x, c: -sin * b.scale_y, d: cos * b.scale_y, e: b.x, f: b.y };
    m.mul(&Mat { e: -ox, f: -oy, ..Mat::ID })
}

/// The local offset ("contain" images centre their photo in the frame).
pub fn offset(l: &Layer) -> (f64, f64) {
    match &l.shape {
        Shape::Image(i) => {
            let (_, _, ox, oy) = natural(&i.url).map_or((0.0, 0.0, 0.0, 0.0), |(w, h)| contain_box(i, w, h));
            (ox, oy)
        }
        _ => (0.0, 0.0),
    }
}

fn natural(url: &str) -> Option<(f64, f64)> {
    image(url).map(|i| (f64::from(i.natural_width()), f64::from(i.natural_height())))
}

thread_local! {
    static MEASURE: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

/// A context only for measuring text, as Konva's dummy context.
pub fn measurer() -> Option<Ctx> {
    MEASURE.with(|m| {
        if m.borrow().is_none() {
            *m.borrow_mut() = new_canvas(1, 1).map(|(_, c)| c);
        }
        m.borrow().clone()
    })
}

pub fn new_canvas(w: u32, h: u32) -> Option<(HtmlCanvasElement, Ctx)> {
    let c: HtmlCanvasElement = leptos::prelude::document().create_element("canvas").ok()?.dyn_into().ok()?;
    c.set_width(w.max(1));
    c.set_height(h.max(1));
    let ctx = c.get_context("2d").ok().flatten()?.dyn_into().ok()?;
    Some((c, ctx))
}

/// `fontStyle fontVariant fontSize px "family"`, as `Text#_getContextFont`.
pub fn font_css(t: &Text) -> String {
    format!("{} normal {}px \"{}\"", t.font_style.css(), t.font_size, t.font_family.replace('"', ""))
}

/// A wrapped line: its text and measured width.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub text: String,
    pub width: f64,
}

/// Konva's `_setTextData`: lines split on `\n`, each wrapped at words to the box width.
pub fn layout_text(t: &Text) -> Vec<TextLine> {
    let Some(ctx) = measurer() else { return Vec::new() };
    ctx.set_font(&font_css(t));
    let graph = |s: &str| s.chars().count();
    let width_of = |s: &str, n: usize| ctx.measure_text(s).map(|m| m.width()).unwrap_or(0.0) + if t.letter_spacing != 0.0 { t.letter_spacing * n as f64 } else { 0.0 };
    let max_width = t.width;
    let mut out = Vec::new();
    for line in t.text.split('\n') {
        let line_width = width_of(line, graph(line));
        if line_width <= max_width {
            out.push(TextLine { text: line.to_string(), width: line_width });
            continue;
        }
        let g: Vec<char> = line.chars().collect();
        let len = g.len();
        let mut start = 0;
        let text = |s: usize, e: usize| g.get(s..e).map(|x| x.iter().collect::<String>()).unwrap_or_default();
        let per_line = ((len as f64 * max_width / line_width).ceil() as usize).max(1);
        let is_break = |c: Option<&char>| matches!(c, Some(' ' | '-'));
        while start < len {
            let mut match_width = 0.0;
            let mut fits = |end: usize| {
                let w = width_of(&text(start, end), end - start);
                if w > max_width {
                    return false;
                }
                match_width = w;
                true
            };
            let (mut low, mut high) = (start, len.min(start + per_line));
            while fits(high) {
                low = high;
                if high == len {
                    break;
                }
                high = len.min(high + (high - start));
            }
            while high - low > 1 {
                let mid = (low + high) / 2;
                if fits(mid) {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            if low == start {
                break;
            }
            if low == len {
                out.push(TextLine { text: text(start, len), width: match_width });
                break;
            }
            if !is_break(g.get(low)) {
                let mut wrap = low as isize - 1;
                while wrap >= start as isize && !is_break(g.get(wrap as usize)) {
                    wrap -= 1;
                }
                if wrap >= start as isize {
                    low = wrap as usize + 1;
                }
            }
            let piece = text(start, low).trim_end().to_string();
            // `_addTextLine` measures the trimmed line again.
            let w = width_of(&piece, graph(&piece));
            out.push(TextLine { text: piece, width: w });
            start = low;
            while g.get(start).is_some_and(|c| c.is_whitespace()) {
                start += 1;
            }
        }
    }
    out
}

/// A text layer's laid-out height: lines × lineHeight × fontSize.
pub fn text_height(t: &Text) -> f64 {
    layout_text(t).len() as f64 * t.line_height * t.font_size
}

/// The layer's own box before its transform (Konva `getSelfRect`).
pub fn self_rect(l: &Layer) -> Rect {
    match &l.shape {
        Shape::Image(i) => {
            let (w, h) = natural(&i.url).map_or((i.width, i.height), |(nw, nh)| {
                let (w, h, _, _) = contain_box(i, nw, nh);
                (w, h)
            });
            Rect { x: 0.0, y: 0.0, width: w, height: h }
        }
        Shape::Text(t) => Rect { x: 0.0, y: 0.0, width: t.width, height: text_height(t) },
        Shape::Rect(r) => Rect { x: 0.0, y: 0.0, width: r.width, height: r.height },
        Shape::Ellipse(e) => Rect { x: -e.radius_x, y: -e.radius_y, width: e.radius_x * 2.0, height: e.radius_y * 2.0 },
        Shape::Polygon(p) => bbox(&polygon_points(p.sides, p.radius)),
        Shape::Star(s) => Rect { x: -s.outer_radius, y: -s.outer_radius, width: s.outer_radius * 2.0, height: s.outer_radius * 2.0 },
        Shape::Line(x) => bbox(&pairs(&x.points)),
        Shape::Curve(c) => bbox(&curve_samples(&c.line.points)),
    }
}

fn stroke_pad(l: &Layer) -> f64 {
    match (l.style(), l.line()) {
        (Some(s), _) if s.stroke_width > 0.0 => s.stroke_width / 2.0,
        (_, Some(x)) => x.stroke_width / 2.0,
        _ => 0.0,
    }
}

/// The layer's axis-aligned box on the page, stroke included (`getClientRect`).
pub fn client_rect(l: &Layer) -> Rect {
    let r = self_rect(l);
    let p = stroke_pad(l);
    let m = node_mat(l);
    let corners = [(r.x - p, r.y - p), (r.x + r.width + p, r.y - p), (r.x + r.width + p, r.y + r.height + p), (r.x - p, r.y + r.height + p)];
    bbox(&corners.iter().map(|(x, y)| m.apply(*x, *y)).collect::<Vec<_>>())
}

pub fn bbox(pts: &[(f64, f64)]) -> Rect {
    if pts.is_empty() {
        return Rect::default();
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (x, y) in pts {
        x0 = x0.min(*x);
        y0 = y0.min(*y);
        x1 = x1.max(*x);
        y1 = y1.max(*y);
    }
    Rect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
}

fn pairs(p: &[f64]) -> Vec<(f64, f64)> {
    p.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

/// Konva `RegularPolygon`: vertex n at (r·sin(2πn/s), −r·cos(2πn/s)).
pub fn polygon_points(sides: f64, r: f64) -> Vec<(f64, f64)> {
    let s = sides.max(3.0) as usize;
    (0..s).map(|n| {
        let a = n as f64 * 2.0 * PI / s as f64;
        (r * a.sin(), -r * a.cos())
    }).collect()
}

pub fn star_points(num: f64, inner: f64, outer: f64) -> Vec<(f64, f64)> {
    let n = num.max(3.0) as usize;
    (0..n * 2).map(|i| {
        let r = if i % 2 == 0 { outer } else { inner };
        let a = i as f64 * PI / n as f64;
        (r * a.sin(), -r * a.cos())
    }).collect()
}

/// A curve as drawn: start, bend (the quadratic control point), end.
pub fn curve_samples(p: &[f64]) -> Vec<(f64, f64)> {
    let pts = pairs(p);
    match pts.as_slice() {
        [s, c, e, ..] => (0..=24)
            .map(|i| {
                let t = f64::from(i) / 24.0;
                let u = 1.0 - t;
                (u * u * s.0 + 2.0 * u * t * c.0 + t * t * e.0, u * u * s.1 + 2.0 * u * t * c.1 + t * t * e.1)
            })
            .collect(),
        _ => pts,
    }
}

fn dist_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn inside_polygon(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    inside
}

/// Whether a page point lands on the layer. Lines get a wide hit stroke, as
/// the editor gave them (`max(32, width + 24, 24 / zoom)`).
pub fn hit(l: &Layer, px: f64, py: f64, zoom: f64) -> bool {
    let p = node_mat(l).invert().apply(px, py);
    let pad = stroke_pad(l);
    match &l.shape {
        Shape::Image(_) | Shape::Text(_) | Shape::Rect(_) => {
            let r = self_rect(l);
            p.0 >= r.x - pad && p.0 <= r.x + r.width + pad && p.1 >= r.y - pad && p.1 <= r.y + r.height + pad
        }
        Shape::Ellipse(e) => {
            let (rx, ry) = (e.radius_x + pad, e.radius_y + pad);
            (p.0 / rx).powi(2) + (p.1 / ry).powi(2) <= 1.0
        }
        Shape::Polygon(x) => inside_polygon(p, &polygon_points(x.sides, x.radius + pad)),
        Shape::Star(s) => inside_polygon(p, &star_points(s.num_points, s.inner_radius + pad, s.outer_radius + pad)),
        Shape::Line(_) | Shape::Curve(_) => {
            let pts = match &l.shape {
                Shape::Curve(c) => curve_samples(&c.line.points),
                Shape::Line(x) => pairs(&x.points),
                _ => Vec::new(),
            };
            let sw = l.line().map_or(4.0, |x| x.stroke_width);
            let half = 32f64.max(sw + 24.0).max(24.0 / zoom) / 2.0;
            pts.windows(2).any(|w| dist_to_segment(p, w[0], w[1]) <= half)
        }
    }
}

fn set_fill(ctx: &Ctx, fill: &Fill, w: f64, h: f64, ox: f64, oy: f64) {
    match fill {
        Fill::Solid { color } => ctx.set_fill_style_str(color),
        Fill::Linear { from, to, angle } => {
            let (x0, y0, x1, y1) = gradient_line(*angle, w, h, ox, oy);
            let g = ctx.create_linear_gradient(x0, y0, x1, y1);
            let _ = g.add_color_stop(0.0, from);
            let _ = g.add_color_stop(1.0, to);
            ctx.set_fill_style_canvas_gradient(&g);
        }
    }
}

fn stroke_style(ctx: &Ctx, fill: &Fill, w: f64, h: f64) {
    match fill {
        Fill::Solid { color } => ctx.set_stroke_style_str(color),
        Fill::Linear { from, to, angle } => {
            let (x0, y0, x1, y1) = gradient_line(*angle, w, h, 0.0, 0.0);
            let g = ctx.create_linear_gradient(x0, y0, x1, y1);
            let _ = g.add_color_stop(0.0, from);
            let _ = g.add_color_stop(1.0, to);
            ctx.set_stroke_style_canvas_gradient(&g);
        }
    }
}

fn fill_stroke(ctx: &Ctx, s: &lattice_core::canvas::Style, w: f64, h: f64, ox: f64, oy: f64) {
    set_fill(ctx, &s.fill, w, h, ox, oy);
    ctx.fill();
    if s.stroke_width > 0.0 {
        ctx.set_line_width(s.stroke_width);
        ctx.set_stroke_style_str(&s.stroke);
        ctx.stroke();
    }
}

fn path(ctx: &Ctx, pts: &[(f64, f64)]) {
    ctx.begin_path();
    for (i, (x, y)) in pts.iter().enumerate() {
        if i == 0 {
            ctx.move_to(*x, *y);
        } else {
            ctx.line_to(*x, *y);
        }
    }
    ctx.close_path();
}

fn draw_text(ctx: &Ctx, t: &Text) {
    if t.text.is_empty() {
        return;
    }
    let lines = layout_text(t);
    let lh = t.line_height * t.font_size;
    ctx.set_font(&font_css(t));
    // Konva's non-legacy text: alphabetic baseline, centred on the line box by the font's ascent/descent.
    let m = ctx.measure_text("M").ok();
    let ascent = m.as_ref().map_or(t.font_size * 0.91, |m| m.font_bounding_box_ascent());
    let descent = m.as_ref().map_or(t.font_size * 0.21, |m| m.font_bounding_box_descent());
    let mut y = (ascent - descent) / 2.0 + lh / 2.0;
    ctx.set_text_baseline("alphabetic");
    ctx.set_text_align("left");
    // The gradient spans width × (fontSize · lineHeight · paragraphs), as fillProps sized it.
    let paragraphs = t.text.split('\n').count() as f64;
    set_fill(ctx, &t.fill, t.width, t.font_size * t.line_height * paragraphs, 0.0, 0.0);
    for line in &lines {
        let mut x = match t.align {
            lattice_core::canvas::Align::Right => t.width - line.width,
            lattice_core::canvas::Align::Center => (t.width - line.width) / 2.0,
            lattice_core::canvas::Align::Left => 0.0,
        };
        if t.text_decoration == Decoration::Underline {
            ctx.save();
            ctx.begin_path();
            let uy = y + (t.font_size / 4.0).round();
            ctx.move_to(x, uy);
            ctx.line_to(x + line.width.round(), uy);
            ctx.set_line_width(t.font_size / 15.0);
            stroke_style(ctx, &t.fill, t.width, t.font_size * t.line_height * paragraphs);
            ctx.stroke();
            ctx.restore();
        }
        if t.letter_spacing != 0.0 {
            for ch in line.text.chars() {
                let s = ch.to_string();
                let _ = ctx.fill_text(&s, x, y);
                x += ctx.measure_text(&s).map(|m| m.width()).unwrap_or(0.0) + t.letter_spacing;
            }
        } else {
            let _ = ctx.fill_text(&line.text, x, y);
        }
        y += lh;
    }
}

/// One layer, under the context's current transform (the page's).
pub fn draw_layer(ctx: &Ctx, l: &Layer) {
    if !l.base.visible {
        return;
    }
    ctx.save();
    let m = node_mat(l);
    let _ = ctx.transform(m.a, m.b, m.c, m.d, m.e, m.f);
    ctx.set_global_alpha(ctx.global_alpha() * l.base.opacity);
    match &l.shape {
        Shape::Image(i) => {
            if let Some(img) = image(&i.url) {
                let (w, h, _, _) = contain_box(i, f64::from(img.natural_width()), f64::from(img.natural_height()));
                let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, w, h);
            }
        }
        Shape::Text(t) => draw_text(ctx, t),
        Shape::Rect(r) => {
            ctx.begin_path();
            let cr = r.corner_radius.min(r.width / 2.0).min(r.height / 2.0).max(0.0);
            if cr == 0.0 {
                ctx.rect(0.0, 0.0, r.width, r.height);
            } else {
                let (w, h) = (r.width, r.height);
                ctx.move_to(cr, 0.0);
                ctx.line_to(w - cr, 0.0);
                let _ = ctx.arc(w - cr, cr, cr, PI * 1.5, 0.0);
                ctx.line_to(w, h - cr);
                let _ = ctx.arc(w - cr, h - cr, cr, 0.0, PI / 2.0);
                ctx.line_to(cr, h);
                let _ = ctx.arc(cr, h - cr, cr, PI / 2.0, PI);
                ctx.line_to(0.0, cr);
                let _ = ctx.arc(cr, cr, cr, PI, PI * 1.5);
            }
            ctx.close_path();
            fill_stroke(ctx, &r.style, r.width, r.height, 0.0, 0.0);
        }
        Shape::Ellipse(e) => {
            ctx.begin_path();
            ctx.save();
            if e.radius_x != e.radius_y {
                let _ = ctx.scale(1.0, e.radius_y / e.radius_x);
            }
            let _ = ctx.arc(0.0, 0.0, e.radius_x, 0.0, PI * 2.0);
            ctx.restore();
            ctx.close_path();
            fill_stroke(ctx, &e.style, e.radius_x * 2.0, e.radius_y * 2.0, -e.radius_x, -e.radius_y);
        }
        Shape::Polygon(p) => {
            path(ctx, &polygon_points(p.sides, p.radius));
            fill_stroke(ctx, &p.style, p.radius * 2.0, p.radius * 2.0, -p.radius, -p.radius);
        }
        Shape::Star(s) => {
            path(ctx, &star_points(s.num_points, s.inner_radius, s.outer_radius));
            fill_stroke(ctx, &s.style, s.outer_radius * 2.0, s.outer_radius * 2.0, -s.outer_radius, -s.outer_radius);
        }
        Shape::Line(_) | Shape::Curve(_) => {
            if let Some(x) = l.line() {
                ctx.begin_path();
                let pts = pairs(&x.points);
                if let Some((sx, sy)) = pts.first() {
                    ctx.move_to(*sx, *sy);
                }
                match (&l.shape, pts.as_slice()) {
                    // Drawn as the quadratic through its bend handle (see the module notes).
                    (Shape::Curve(_), [_, c, e, ..]) => ctx.quadratic_curve_to(c.0, c.1, e.0, e.1),
                    _ => {
                        for (px, py) in pts.iter().skip(1) {
                            ctx.line_to(*px, *py);
                        }
                    }
                }
                let dash = js_sys::Array::new();
                for d in x.dash.iter().flatten() {
                    dash.push(&JsValue::from_f64(*d));
                }
                let _ = ctx.set_line_dash(&dash);
                ctx.set_line_cap(x.line_cap.css());
                ctx.set_line_join("round");
                ctx.set_line_width(x.stroke_width);
                ctx.set_stroke_style_str(&x.stroke);
                ctx.stroke();
            }
        }
    }
    ctx.restore();
}

/// The page itself, at the context's transform: background, its photo, then
/// every visible layer (`skip` leaves layers out, e.g. the one being typed
/// into, or text for the PowerPoint pictures).
pub fn draw_page(ctx: &Ctx, c: &Canvas, skip: &dyn Fn(&Layer) -> bool) {
    ctx.begin_path();
    ctx.rect(0.0, 0.0, c.width, c.height);
    set_fill(ctx, &c.background.fill, c.width, c.height, 0.0, 0.0);
    ctx.fill();
    if let Some(img) = c.background.image_url.as_deref().and_then(image) {
        let (x, y, w, h) = background_cover(c.width, c.height, f64::from(img.natural_width()), f64::from(img.natural_height()));
        let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, x, y, w, h);
    }
    for l in &c.layers {
        if !skip(l) {
            draw_layer(ctx, l);
        }
    }
}

/// Every image and font the page needs, loaded (unreachable images are left out).
pub async fn prepare(c: &Canvas) {
    load_fonts(c).await;
    let urls: Vec<String> = c
        .background
        .image_url
        .iter()
        .cloned()
        .chain(c.layers.iter().filter(|l| l.base.visible).filter_map(|l| match &l.shape {
            Shape::Image(i) => Some(i.url.clone()),
            _ => None,
        }))
        .collect();
    for u in urls {
        let _ = load_image(&u).await;
    }
}

/// The page drawn off-screen at `pixel_ratio`, plus each text layer's laid-out
/// height (for the PowerPoint text boxes). `skip_text` leaves text out.
pub async fn render(c: &Canvas, pixel_ratio: f64, skip_text: bool) -> Result<(HtmlCanvasElement, Vec<(Layer, f64)>), ApiError> {
    prepare(c).await;
    let (w, h) = ((c.width * pixel_ratio).round() as u32, (c.height * pixel_ratio).round() as u32);
    let (el, ctx) = new_canvas(w, h).ok_or_else(|| ApiError::Server("No canvas to draw on.".into()))?;
    let _ = ctx.scale(pixel_ratio, pixel_ratio);
    draw_page(&ctx, c, &|l| skip_text && l.kind() == "text");
    let texts = c
        .layers
        .iter()
        .filter(|l| l.base.visible)
        .filter_map(|l| match &l.shape {
            Shape::Text(t) => Some((l.clone(), text_height(t))),
            _ => None,
        })
        .collect();
    Ok((el, texts))
}

/// The page as PNG bytes.
pub async fn render_png(c: &Canvas, pixel_ratio: f64, skip_text: bool) -> Result<(Vec<u8>, Vec<(Layer, f64)>), ApiError> {
    let (el, texts) = render(c, pixel_ratio, skip_text).await?;
    let blob = crate::photos::to_blob(&el, "image/png", None)
        .await
        .map_err(|_| ApiError::Invalid("Export blocked by an image from another website. Upload that image to the product instead.".into()))?
        .ok_or_else(|| ApiError::Invalid("Export blocked by an image from another website. Upload that image to the product instead.".into()))?;
    Ok((crate::photos::blob_bytes(&blob).await?, texts))
}

/// A data URL of the page at `pixel_ratio` (thumbnails).
pub async fn render_data_url(c: &Canvas, pixel_ratio: f64) -> Option<String> {
    let (el, _) = render(c, pixel_ratio, false).await.ok()?;
    el.to_data_url().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms() {
        let m = Mat { a: 0.0, b: 1.0, c: -1.0, d: 0.0, e: 10.0, f: 20.0 };
        let (x, y) = m.apply(1.0, 0.0);
        assert_eq!((x, y), (10.0, 21.0));
        let back = m.invert().apply(x, y);
        assert!((back.0 - 1.0).abs() < 1e-12 && back.1.abs() < 1e-12);
        assert!(inside_polygon((0.0, 0.0), &polygon_points(3.0, 10.0)));
        assert!(!inside_polygon((20.0, 0.0), &polygon_points(3.0, 10.0)));
        assert!((dist_to_segment((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)) - 3.0).abs() < 1e-12);
        let c = curve_samples(&[0.0, 0.0, 100.0, -100.0, 200.0, 0.0]);
        assert_eq!(c.first(), Some(&(0.0, 0.0)));
        assert_eq!(c.last(), Some(&(200.0, 0.0)));
        assert!((c[12].1 + 50.0).abs() < 1e-9, "a quadratic peaks halfway to its control point");
    }
}
