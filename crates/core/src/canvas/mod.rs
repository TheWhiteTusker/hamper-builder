//! The designer's document: hamper images and presentation slides (port of
//! `hamper-canvas.ts`, `hamper-canvas-schema.ts`, `canvas-render.ts`).
//!
//! Stored as-is in `hampers.canvas` and `presentation_slides.canvas`, so this
//! JSON shape is the contract with the old app: same names, same defaults
//! for fields added after the first saved designs, and a design that doesn't
//! validate opens as a blank page, as `parseCanvas` did. Layer order is
//! z-order: the last layer is drawn on top.

pub mod arrange;
pub mod cutout;
pub mod deck;
pub mod history;
pub mod magic;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fill {
    Solid { color: String },
    Linear { from: String, to: String, angle: f64 },
}

impl Fill {
    pub fn solid(color: &str) -> Self {
        Self::Solid { color: color.into() }
    }

    /// The colour a single-colour control shows: the solid colour, or a gradient's start.
    pub fn main_color(&self) -> &str {
        match self {
            Self::Solid { color } => color,
            Self::Linear { from, .. } => from,
        }
    }
}

const fn yes() -> bool {
    true
}
const fn one() -> f64 {
    1.0
}
const fn four() -> f64 {
    4.0
}
const fn half() -> f64 {
    0.5
}

/// What every layer has. `visible`, `locked` and `name` came after the first
/// saved designs, hence the defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub opacity: f64,
}

impl Base {
    pub fn at(id: String, x: f64, y: f64) -> Self {
        Self { id, name: None, visible: true, locked: false, x, y, rotation: 0.0, scale_x: 1.0, scale_y: 1.0, opacity: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    #[default]
    Stretch,
    /// Fits the photo inside the width×height frame without stretching.
    Contain,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    #[default]
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "bold")]
    Bold,
    #[serde(rename = "italic")]
    Italic,
    #[serde(rename = "italic bold")]
    ItalicBold,
}

impl FontStyle {
    pub fn bold(self) -> bool {
        matches!(self, Self::Bold | Self::ItalicBold)
    }
    pub fn italic(self) -> bool {
        matches!(self, Self::Italic | Self::ItalicBold)
    }
    pub fn of(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => Self::Normal,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::ItalicBold,
        }
    }
    pub fn css(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::ItalicBold => "italic bold",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decoration {
    #[default]
    None,
    Underline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    #[default]
    Round,
    Butt,
    Square,
}

impl LineCap {
    pub fn css(self) -> &'static str {
        match self {
            Self::Round => "round",
            Self::Butt => "butt",
            Self::Square => "square",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub product_id: Option<String>,
    pub url: String,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub fit: Fit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
    pub text: String,
    pub font_size: f64,
    pub font_family: String,
    pub font_style: FontStyle,
    pub align: Align,
    pub width: f64,
    #[serde(default)]
    pub letter_spacing: f64,
    #[serde(default = "one")]
    pub line_height: f64,
    #[serde(default)]
    pub text_decoration: Decoration,
    pub fill: Fill,
}

/// Fill and outline, shared by the closed shapes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    pub fill: Fill,
    pub stroke: String,
    pub stroke_width: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    #[serde(flatten)]
    pub style: Style,
    pub width: f64,
    pub height: f64,
    pub corner_radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ellipse {
    #[serde(flatten)]
    pub style: Style,
    pub radius_x: f64,
    pub radius_y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Polygon {
    #[serde(flatten)]
    pub style: Style,
    pub sides: f64,
    pub radius: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Star {
    #[serde(flatten)]
    pub style: Style,
    pub num_points: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
}

/// A straight line (`points` x1,y1,x2,y2) or a curve (start, bend, end).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub points: Vec<f64>,
    pub stroke: String,
    #[serde(default = "four")]
    pub stroke_width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dash: Option<Vec<f64>>,
    #[serde(default)]
    pub line_cap: LineCap,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Curve {
    #[serde(flatten)]
    pub line: Line,
    #[serde(default = "half")]
    pub curvature: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Shape {
    Image(Image),
    Text(Text),
    Rect(Rect),
    Ellipse(Ellipse),
    Polygon(Polygon),
    Star(Star),
    Line(Line),
    Curve(Curve),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    #[serde(flatten)]
    pub base: Base,
    #[serde(flatten)]
    pub shape: Shape,
}

impl Layer {
    pub fn id(&self) -> &str {
        &self.base.id
    }

    pub fn style(&self) -> Option<&Style> {
        match &self.shape {
            Shape::Rect(r) => Some(&r.style),
            Shape::Ellipse(e) => Some(&e.style),
            Shape::Polygon(p) => Some(&p.style),
            Shape::Star(s) => Some(&s.style),
            _ => None,
        }
    }

    pub fn style_mut(&mut self) -> Option<&mut Style> {
        match &mut self.shape {
            Shape::Rect(r) => Some(&mut r.style),
            Shape::Ellipse(e) => Some(&mut e.style),
            Shape::Polygon(p) => Some(&mut p.style),
            Shape::Star(s) => Some(&mut s.style),
            _ => None,
        }
    }

    /// A line or a curve's stroke.
    pub fn line(&self) -> Option<&Line> {
        match &self.shape {
            Shape::Line(l) => Some(l),
            Shape::Curve(c) => Some(&c.line),
            _ => None,
        }
    }

    pub fn line_mut(&mut self) -> Option<&mut Line> {
        match &mut self.shape {
            Shape::Line(l) => Some(l),
            Shape::Curve(c) => Some(&mut c.line),
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self.shape {
            Shape::Image(_) => "image",
            Shape::Text(_) => "text",
            Shape::Rect(_) => "rect",
            Shape::Ellipse(_) => "ellipse",
            Shape::Polygon(_) => "polygon",
            Shape::Star(_) => "star",
            Shape::Line(_) => "line",
            Shape::Curve(_) => "curve",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Background {
    pub fill: Fill,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    pub width: f64,
    pub height: f64,
    pub background: Background,
    pub layers: Vec<Layer>,
}

pub const PRESETS: [(&str, f64, f64); 5] = [
    ("Square 1:1", 1080.0, 1080.0),
    ("Portrait 4:5", 1080.0, 1350.0),
    ("Story 9:16", 1080.0, 1920.0),
    ("Landscape 16:9", 1920.0, 1080.0),
    ("A4 portrait", 1240.0, 1754.0),
];

pub fn empty_canvas() -> Canvas {
    Canvas {
        width: PRESETS[0].1,
        height: PRESETS[0].2,
        background: Background { fill: Fill::solid("#ffffff"), image_url: None },
        layers: Vec::new(),
    }
}

fn is_color(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7 && b.first() == Some(&b'#') && b.iter().skip(1).all(u8::is_ascii_hexdigit)
}

/// What `new URL()` accepts, near enough: a scheme, then something.
fn is_url(s: &str) -> bool {
    match s.split_once(':') {
        Some((scheme, rest)) => {
            !rest.is_empty()
                && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
        }
        None => false,
    }
}

fn fill_ok(f: &Fill) -> bool {
    match f {
        Fill::Solid { color } => is_color(color),
        Fill::Linear { from, to, angle } => is_color(from) && is_color(to) && angle.is_finite(),
    }
}

fn int_in(v: f64, lo: f64, hi: f64) -> bool {
    v.fract() == 0.0 && (lo..=hi).contains(&v)
}

impl Layer {
    /// The zod schema's rules.
    fn valid(&self) -> bool {
        let b = &self.base;
        let finite = [b.x, b.y, b.rotation, b.scale_x, b.scale_y].iter().all(|v| v.is_finite());
        let style_ok = |s: &Style| fill_ok(&s.fill) && is_color(&s.stroke) && s.stroke_width >= 0.0;
        let line_ok = |l: &Line| is_color(&l.stroke) && l.stroke_width >= 1.0 && l.points.iter().all(|p| p.is_finite());
        !b.id.is_empty()
            && finite
            && (0.0..=1.0).contains(&b.opacity)
            && match &self.shape {
                Shape::Image(i) => is_url(&i.url) && i.width > 0.0 && i.height > 0.0,
                Shape::Text(t) => t.font_size > 0.0 && t.width > 0.0 && t.line_height > 0.0 && fill_ok(&t.fill),
                Shape::Rect(r) => style_ok(&r.style) && r.width > 0.0 && r.height > 0.0 && r.corner_radius >= 0.0,
                Shape::Ellipse(e) => style_ok(&e.style) && e.radius_x > 0.0 && e.radius_y > 0.0,
                Shape::Polygon(p) => style_ok(&p.style) && int_in(p.sides, 3.0, 12.0) && p.radius > 0.0,
                Shape::Star(s) => {
                    style_ok(&s.style) && int_in(s.num_points, 3.0, 20.0) && s.inner_radius > 0.0 && s.outer_radius > 0.0
                }
                Shape::Line(l) => line_ok(l),
                Shape::Curve(c) => line_ok(&c.line) && c.curvature.is_finite(),
            }
    }
}

impl Canvas {
    pub fn valid(&self) -> bool {
        self.width > 0.0
            && self.height > 0.0
            && fill_ok(&self.background.fill)
            && self.background.image_url.as_deref().is_none_or(is_url)
            && self.layers.iter().all(Layer::valid)
    }
}

/// A saved canvas, or a blank one if there is none or it no longer parses.
pub fn parse_canvas(raw: &serde_json::Value) -> Canvas {
    serde_json::from_value::<Canvas>(raw.clone()).ok().filter(Canvas::valid).unwrap_or_else(empty_canvas)
}

/// What the Layers panel calls a layer when it has not been renamed.
pub fn layer_label(l: &Layer) -> String {
    if let Some(n) = &l.base.name {
        return n.clone();
    }
    match &l.shape {
        Shape::Image(_) => "Image".into(),
        Shape::Text(t) => {
            // `.slice(0, 40)` counts UTF-16 units.
            let units: Vec<u16> = t.text.trim().split('\n').next().unwrap_or("").encode_utf16().take(40).collect();
            let first = String::from_utf16_lossy(&units);
            if first.is_empty() { "Text".into() } else { first }
        }
        Shape::Rect(r) => if r.corner_radius > 0.0 { "Rounded rectangle" } else { "Rectangle" }.into(),
        Shape::Ellipse(_) => "Circle".into(),
        Shape::Polygon(p) => match p.sides as i64 {
            3 => "Triangle".into(),
            6 => "Hexagon".into(),
            n => format!("{n}-sided shape"),
        },
        Shape::Star(_) => "Star".into(),
        Shape::Line(_) => "Line".into(),
        Shape::Curve(_) => "Curved line".into(),
    }
}

/// The fill as CSS, for swatches and layer thumbnails.
pub fn fill_css(fill: &Fill) -> String {
    match fill {
        Fill::Solid { color } => color.clone(),
        Fill::Linear { from, to, angle } => format!("linear-gradient({}deg, {from}, {to})", angle + 90.0),
    }
}

/// Colours already used in the design, offered first in colour pickers.
pub fn document_colors(c: &Canvas) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    };
    let add_fill = |f: &Fill, add: &mut dyn FnMut(&str)| match f {
        Fill::Solid { color } => add(color),
        Fill::Linear { from, to, .. } => {
            add(from);
            add(to);
        }
    };
    add_fill(&c.background.fill, &mut add);
    for l in &c.layers {
        match &l.shape {
            Shape::Image(_) => {}
            Shape::Text(t) => add_fill(&t.fill, &mut add),
            Shape::Line(x) | Shape::Curve(Curve { line: x, .. }) => {
                if x.stroke_width > 0.0 {
                    add(&x.stroke);
                }
            }
            _ => {
                if let Some(s) = l.style() {
                    add_fill(&s.fill, &mut add);
                    if s.stroke_width > 0.0 {
                        add(&s.stroke);
                    }
                }
            }
        }
    }
    out
}

/// A gradient line through a box's centre at `angle` degrees (0 = left→right,
/// 90 = top→bottom), long enough to reach the corners: (x0, y0, x1, y1).
/// The box starts at (ox, oy); centred shapes pass -rx, -ry.
pub fn gradient_line(angle: f64, w: f64, h: f64, ox: f64, oy: f64) -> (f64, f64, f64, f64) {
    let rad = angle * std::f64::consts::PI / 180.0;
    let (dx, dy) = (rad.cos(), rad.sin());
    let half = (dx.abs() * w + dy.abs() * h) / 2.0;
    let (cx, cy) = (ox + w / 2.0, oy + h / 2.0);
    (cx - dx * half, cy - dy * half, cx + dx * half, cy + dy * half)
}

/// An image layer's drawn box with "contain": (width, height, offset_x,
/// offset_y). Offsets are local, so rotation and scale still pivot on the
/// frame corner.
pub fn contain_box(i: &Image, natural_w: f64, natural_h: f64) -> (f64, f64, f64, f64) {
    if i.fit != Fit::Contain || natural_w <= 0.0 || natural_h <= 0.0 {
        return (i.width, i.height, 0.0, 0.0);
    }
    let k = (i.width / natural_w).min(i.height / natural_h);
    let (w, h) = (natural_w * k, natural_h * k);
    (w, h, -(i.width - w) / 2.0, -(i.height - h) / 2.0)
}

/// A background image scaled to cover the whole page, centred: (x, y, w, h).
pub fn background_cover(page_w: f64, page_h: f64, natural_w: f64, natural_h: f64) -> (f64, f64, f64, f64) {
    let k = (page_w / natural_w).max(page_h / natural_h);
    let (w, h) = (natural_w * k, natural_h * k);
    ((page_w - w) / 2.0, (page_h - h) / 2.0, w, h)
}

/// The fonts the studio offers; `google` is the Google Fonts family spec.
pub const FONTS: [(&str, Option<&str>); 18] = [
    ("Playfair Display", Some("Playfair+Display:ital,wght@0,400;0,700;1,400;1,700")),
    ("Cormorant Garamond", Some("Cormorant+Garamond:ital,wght@0,400;0,700;1,400;1,700")),
    ("Lora", Some("Lora:ital,wght@0,400;0,700;1,400;1,700")),
    ("Cinzel", Some("Cinzel:wght@400;700")),
    ("Montserrat", Some("Montserrat:ital,wght@0,400;0,700;1,400;1,700")),
    ("Poppins", Some("Poppins:ital,wght@0,400;0,700;1,400;1,700")),
    ("Inter", Some("Inter:ital,wght@0,400;0,700;1,400;1,700")),
    ("Questrial", Some("Questrial")),
    ("Cabin", Some("Cabin:ital,wght@0,400;0,700;1,400;1,700")),
    ("Oswald", Some("Oswald:wght@400;700")),
    ("Bebas Neue", Some("Bebas+Neue")),
    ("Great Vibes", Some("Great+Vibes")),
    ("Dancing Script", Some("Dancing+Script:wght@400;700")),
    ("Pacifico", Some("Pacifico")),
    ("Arial", None),
    ("Georgia", None),
    ("Times New Roman", None),
    ("Courier New", None),
];

/// The Google Fonts stylesheet for every web font above.
pub fn fonts_href() -> String {
    let families: Vec<String> = FONTS.iter().filter_map(|(_, g)| g.map(|g| format!("family={g}"))).collect();
    format!("https://fonts.googleapis.com/css2?{}&display=swap", families.join("&"))
}

#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub font_family: &'static str,
    pub font_size: f64,
    pub font_style: FontStyle,
    pub color: &'static str,
    pub letter_spacing: f64,
}

const fn ts(font_family: &'static str, font_size: f64, font_style: FontStyle, color: &'static str, letter_spacing: f64) -> TextStyle {
    TextStyle { font_family, font_size, font_style, color, letter_spacing }
}

/// The Text panel's three sizes: (label, text, style, preview px).
pub const TEXT_PRESETS: [(&str, TextStyle, f64); 3] = [
    ("Add a heading", ts("Playfair Display", 96.0, FontStyle::Bold, "#2c332f", 0.0), 22.0),
    ("Add a subheading", ts("Montserrat", 56.0, FontStyle::Bold, "#2c332f", 0.0), 16.0),
    ("Add a little bit of body text", ts("Inter", 34.0, FontStyle::Normal, "#2c332f", 0.0), 12.0),
];

pub const TEXT_COMBOS: [(&str, TextStyle); 6] = [
    ("Happy Diwali", ts("Great Vibes", 130.0, FontStyle::Normal, "#b08d57", 0.0)),
    ("SEASON'S GREETINGS", ts("Cinzel", 64.0, FontStyle::Bold, "#2c332f", 6.0)),
    ("With love", ts("Dancing Script", 110.0, FontStyle::Bold, "#c0392b", 0.0)),
    ("THE GOURMET BOX", ts("Bebas Neue", 120.0, FontStyle::Normal, "#54655b", 4.0)),
    ("Thank you", ts("Pacifico", 96.0, FontStyle::Normal, "#ff66c4", 0.0)),
    ("Curated with care", ts("Cormorant Garamond", 72.0, FontStyle::Italic, "#2c332f", 0.0)),
];

/// A text layer centred on the page, 80% of its width.
pub fn make_text(page: &Canvas, id: String, text: &str, s: &TextStyle) -> Layer {
    let width = page.width * 0.8;
    Layer {
        base: Base::at(id, (page.width - width) / 2.0, page.height / 2.0 - s.font_size / 2.0),
        shape: Shape::Text(Text {
            text: text.into(),
            font_size: s.font_size,
            font_family: s.font_family.into(),
            font_style: s.font_style,
            align: Align::Center,
            width,
            letter_spacing: s.letter_spacing,
            line_height: 1.1,
            text_decoration: Decoration::None,
            fill: Fill::solid(s.color),
        }),
    }
}

/// The Elements panel's shapes, each made at the centre of the page: (label, svg, make).
pub fn element(label: &str, page: &Canvas, id: String) -> Option<Layer> {
    let (cx, cy) = (page.width / 2.0, page.height / 2.0);
    let style = || Style { fill: Fill::solid("#c8a97e"), stroke: "#2c332f".into(), stroke_width: 0.0 };
    let line = |dash: Option<Vec<f64>>| Line { points: vec![0.0, 0.0, 400.0, 0.0], stroke: "#2c332f".into(), stroke_width: 4.0, dash, line_cap: LineCap::Round };
    let (x, y, shape) = match label {
        "Square" => (cx - 150.0, cy - 150.0, Shape::Rect(Rect { style: style(), width: 300.0, height: 300.0, corner_radius: 0.0 })),
        "Rounded square" => (cx - 150.0, cy - 150.0, Shape::Rect(Rect { style: style(), width: 300.0, height: 300.0, corner_radius: 48.0 })),
        "Rectangle" => (cx - 250.0, cy - 100.0, Shape::Rect(Rect { style: style(), width: 500.0, height: 200.0, corner_radius: 0.0 })),
        "Circle" => (cx, cy, Shape::Ellipse(Ellipse { style: style(), radius_x: 150.0, radius_y: 150.0 })),
        "Triangle" => (cx, cy, Shape::Polygon(Polygon { style: style(), sides: 3.0, radius: 170.0 })),
        "Pentagon" => (cx, cy, Shape::Polygon(Polygon { style: style(), sides: 5.0, radius: 160.0 })),
        "Hexagon" => (cx, cy, Shape::Polygon(Polygon { style: style(), sides: 6.0, radius: 160.0 })),
        "Star" => (cx, cy, Shape::Star(Star { style: style(), num_points: 5.0, inner_radius: 70.0, outer_radius: 170.0 })),
        "Burst" => (cx, cy, Shape::Star(Star { style: style(), num_points: 12.0, inner_radius: 130.0, outer_radius: 170.0 })),
        "Line" => (cx - 200.0, cy, Shape::Line(line(None))),
        "Dashed line" => (cx - 200.0, cy, Shape::Line(line(Some(vec![16.0, 12.0])))),
        "Curved line" => (
            cx - 200.0,
            cy,
            Shape::Curve(Curve { line: Line { points: vec![0.0, 0.0, 200.0, -120.0, 400.0, 0.0], ..line(None) }, curvature: 0.5 }),
        ),
        _ => return None,
    };
    Some(Layer { base: Base::at(id, x, y), shape })
}

/// The Elements panel's tiles: (label, inline SVG).
pub const ELEMENTS: [(&str, &str); 12] = [
    ("Square", r#"<rect x="4" y="4" width="40" height="40"/>"#),
    ("Rounded square", r#"<rect x="4" y="4" width="40" height="40" rx="10"/>"#),
    ("Rectangle", r#"<rect x="2" y="12" width="44" height="24"/>"#),
    ("Circle", r#"<circle cx="24" cy="24" r="20"/>"#),
    ("Triangle", r#"<polygon points="24,4 45,42 3,42"/>"#),
    ("Pentagon", r#"<polygon points="24,3 45,19 37,44 11,44 3,19"/>"#),
    ("Hexagon", r#"<polygon points="24,3 43,14 43,34 24,45 5,34 5,14"/>"#),
    ("Star", r#"<polygon points="24,3 30,18 46,18 33,28 38,44 24,34 10,44 15,28 2,18 18,18"/>"#),
    ("Burst", r#"<polygon points="24,2 28,15 40,8 33,20 46,24 33,28 40,40 28,33 24,46 20,33 8,40 15,28 2,24 15,20 8,8 20,15"/>"#),
    ("Line", r#"<line x1="6" y1="24" x2="42" y2="24" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>"#),
    ("Curved line", r#"<path d="M6 34 Q24 10 42 34" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>"#),
    ("Dashed line", r#"<line x1="6" y1="24" x2="42" y2="24" stroke="currentColor" stroke-width="4" stroke-linecap="round" stroke-dasharray="6,4"/>"#),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text_layer() -> Layer {
        Layer {
            base: Base { rotation: 15.0, ..Base::at("t1".into(), 1.0, 2.0) },
            shape: Shape::Text(Text {
                text: "Hi".into(),
                font_size: 40.0,
                font_family: "Arial".into(),
                font_style: FontStyle::Bold,
                align: Align::Center,
                width: 300.0,
                letter_spacing: 0.0,
                line_height: 1.0,
                text_decoration: Decoration::None,
                fill: Fill::Linear { from: "#111111".into(), to: "#eeeeee".into(), angle: 45.0 },
            }),
        }
    }

    #[test]
    fn round_trips_and_falls_back_on_junk() {
        let mut doc = empty_canvas();
        doc.layers.push(text_layer());
        doc.layers.push(Layer {
            base: Base { scale_x: 2.0, scale_y: 2.0, opacity: 0.5, ..Base::at("i1".into(), 0.0, 0.0) },
            shape: Shape::Image(Image { product_id: None, url: "https://example.com/a.png".into(), width: 200.0, height: 100.0, fit: Fit::Stretch }),
        });
        let raw = serde_json::to_value(&doc).unwrap();
        assert_eq!(raw["layers"][0]["kind"], "text");
        assert_eq!(raw["layers"][0]["fontStyle"], "bold");
        assert_eq!(raw["layers"][1]["product_id"], serde_json::Value::Null);
        assert_eq!(parse_canvas(&raw), doc);
        assert_eq!(parse_canvas(&serde_json::Value::Null), empty_canvas());
        assert_eq!(parse_canvas(&json!({ "layers": "nope" })), empty_canvas());
        let mut bad = raw.clone();
        bad["layers"][0]["fill"]["from"] = json!("red");
        assert_eq!(parse_canvas(&bad), empty_canvas(), "a colour that isn't #rrggbb fails the whole design, as zod did");
    }

    #[test]
    fn old_designs_get_the_new_defaults() {
        let old = json!({
            "width": 1080, "height": 1080, "background": { "fill": { "type": "solid", "color": "#ffffff" }, "image_url": null },
            "layers": [{ "id": "t", "kind": "text", "x": 0, "y": 0, "rotation": 0, "scaleX": 1, "scaleY": 1, "opacity": 1,
                "text": "Hi", "fontSize": 10, "fontFamily": "Arial", "fontStyle": "normal", "align": "left", "width": 50,
                "fill": { "type": "solid", "color": "#000000" } },
              { "id": "l1", "kind": "line", "x": 50, "y": 60, "rotation": 0, "scaleX": 1, "scaleY": 1, "opacity": 1,
                "points": [0, 0, 200, 100], "stroke": "#2c332f" },
              { "id": "c1", "kind": "curve", "x": 50, "y": 60, "rotation": 0, "scaleX": 1, "scaleY": 1, "opacity": 1,
                "points": [0, 0, 100, -50, 200, 0], "stroke": "#54655b", "strokeWidth": 6 }]
        });
        let c = parse_canvas(&old);
        let t = &c.layers[0];
        assert!(t.base.visible && !t.base.locked);
        assert!(matches!(&t.shape, Shape::Text(x) if x.line_height == 1.0 && x.text_decoration == Decoration::None));
        assert_eq!(layer_label(t), "Hi");
        assert!(matches!(&c.layers[1].shape, Shape::Line(l) if l.stroke_width == 4.0 && l.line_cap == LineCap::Round));
        assert!(matches!(&c.layers[2].shape, Shape::Curve(k) if k.curvature == 0.5 && k.line.stroke_width == 6.0));
    }

    #[test]
    fn gradients_span_the_box_and_css_matches() {
        assert_eq!(gradient_line(0.0, 100.0, 50.0, 0.0, 0.0), (0.0, 25.0, 100.0, 25.0));
        let (x0, y0, _, y1) = gradient_line(90.0, 40.0, 60.0, -20.0, -30.0);
        assert!(x0.abs() < 1e-9 && (y0 + 30.0).abs() < 1e-9 && (y1 - 30.0).abs() < 1e-9);
        assert_eq!(fill_css(&Fill::Linear { from: "#000000".into(), to: "#ffffff".into(), angle: 0.0 }), "linear-gradient(90deg, #000000, #ffffff)");
    }

    #[test]
    fn contain_and_cover() {
        let i = Image { product_id: None, url: "https://e.com/a.png".into(), width: 400.0, height: 400.0, fit: Fit::Contain };
        assert_eq!(contain_box(&i, 100.0, 200.0), (200.0, 400.0, -100.0, 0.0));
        assert_eq!(contain_box(&Image { fit: Fit::Stretch, ..i }, 100.0, 200.0), (400.0, 400.0, 0.0, 0.0));
        assert_eq!(background_cover(1000.0, 500.0, 100.0, 100.0), (0.0, -250.0, 1000.0, 1000.0));
    }

    #[test]
    fn labels() {
        let p = |sides| Layer { base: Base::at("p".into(), 0.0, 0.0), shape: Shape::Polygon(Polygon { style: Style { fill: Fill::solid("#000000"), stroke: "#000000".into(), stroke_width: 0.0 }, sides, radius: 1.0 }) };
        assert_eq!(layer_label(&p(3.0)), "Triangle");
        assert_eq!(layer_label(&p(5.0)), "5-sided shape");
        let blank = element("Square", &empty_canvas(), "x".into());
        assert_eq!(blank.map(|l| layer_label(&l)).as_deref(), Some("Rectangle"));
    }
}
