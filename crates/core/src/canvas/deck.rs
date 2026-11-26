//! Presentation decks in the house format (port of `presentation.ts`,
//! `deck-*.ts`): a green entry slide, one white slide per hamper or product
//! with the photos standing on a green band, and a green thank-you slide.
//! Every slide is an ordinary designer canvas, so the editor edits them after.
//! The layout needs each photo's real proportions, so it runs once the images
//! have loaded.

use super::{Align, Background, Base, Canvas, Decoration, Fill, FontStyle, Image, Layer, Rect, Shape, Style, Text};
use serde::{Deserialize, Serialize};

pub const SLIDE_W: f64 = 1920.0;
pub const SLIDE_H: f64 = 1080.0;

pub mod colors {
    /// The house deck's dusty pink, in Lattice Lane sage.
    pub const GREEN: &str = "#b5c2b1";
    pub const HEADING: &str = "#595959";
    pub const INK: &str = "#1f1f1f";
    pub const MUTED: &str = "#7f7f7f";
    pub const PAGE: &str = "#ffffff";
}

pub const DEFAULT_NOTE: &str = "Pricing is applicable for an MOQ of 100 units. Estimated delivery: 15 working days from design approval and advance payment confirmation.";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Photo {
    pub url: String,
    pub size: Size,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckItemInfo {
    pub product_id: Option<String>,
    pub name: String,
    /// Small line under the name, e.g. the product's category.
    pub caption: String,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HamperInfo {
    pub id: String,
    pub code: String,
    pub name: String,
    pub price: Option<f64>,
    /// The hamper's designed picture, used when no product has a photo.
    pub image_url: Option<String>,
    pub items: Vec<DeckItemInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductInfo {
    pub id: String,
    pub code: String,
    pub name: String,
    pub caption: String,
    pub price: Option<f64>,
    pub image_url: Option<String>,
}

/// Fresh layer ids, from the caller (the UI hands out UUIDs).
pub type Ids<'a> = &'a mut dyn FnMut() -> String;

struct TextOpts<'a> {
    size: f64,
    color: &'a str,
    width: f64,
    font: &'a str,
    style: FontStyle,
    align: Align,
    spacing: f64,
    line_height: f64,
    underline: bool,
    name: &'a str,
}

impl Default for TextOpts<'_> {
    fn default() -> Self {
        Self {
            size: 20.0,
            color: colors::INK,
            width: 100.0,
            font: "Arial",
            style: FontStyle::Normal,
            align: Align::Left,
            spacing: 0.0,
            line_height: 1.15,
            underline: false,
            name: "",
        }
    }
}

fn named(ids: Ids, x: f64, y: f64, name: &str) -> Base {
    Base { name: (!name.is_empty()).then(|| name.to_string()), ..Base::at(ids(), x, y) }
}

fn text(ids: Ids, value: &str, x: f64, y: f64, o: TextOpts) -> Layer {
    Layer {
        base: named(ids, x, y, o.name),
        shape: Shape::Text(Text {
            text: value.into(),
            font_size: o.size,
            font_family: o.font.into(),
            font_style: o.style,
            align: o.align,
            width: o.width,
            letter_spacing: o.spacing,
            line_height: o.line_height,
            text_decoration: if o.underline { Decoration::Underline } else { Decoration::None },
            fill: Fill::solid(o.color),
        }),
    }
}

fn rect(ids: Ids, x: f64, y: f64, width: f64, height: f64, color: &str, name: &str) -> Layer {
    Layer {
        base: named(ids, x, y, name),
        shape: Shape::Rect(Rect {
            style: Style { fill: Fill::solid(color), stroke: colors::INK.into(), stroke_width: 0.0 },
            width,
            height,
            corner_radius: 0.0,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn photo(
    ids: Ids,
    url: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    product_id: Option<String>,
    name: &str,
) -> Layer {
    Layer {
        base: named(ids, x, y, name),
        shape: Shape::Image(Image { product_id, url: url.into(), width, height, fit: super::Fit::Stretch }),
    }
}

fn page(background: &str, layers: Vec<Layer>) -> Canvas {
    Canvas {
        width: SLIDE_W,
        height: SLIDE_H,
        background: Background { fill: Fill::solid(background), image_url: None },
        layers,
    }
}

/// "Essential Hamper" → ("Essential", "Hamper"), as the deck sets its titles.
pub fn split_title(name: &str, fallback_subtitle: &str) -> (String, String) {
    let n = name.trim();
    let lower = n.to_lowercase();
    for word in ["hampers", "hamper"] {
        if let Some(head) = lower.strip_suffix(word)
            && head.ends_with(char::is_whitespace)
            && !head.trim().is_empty()
        {
            let title = n.get(..head.trim_end().len()).unwrap_or(n).to_string();
            let mut w = word.to_string();
            w.replace_range(..1, "H");
            return (title, w);
        }
    }
    (n.to_string(), fallback_subtitle.to_string())
}

/// "INR 1,25,000": rounded rupees, Indian grouping.
pub fn format_inr(v: f64) -> String {
    let r = crate::js::round(v);
    let sign = if r < 0.0 { "-" } else { "" };
    format!("INR {sign}{}", crate::money::group_indian(&format!("{:.0}", r.abs())))
}

/// The logo, `width` px wide, tucked into a corner.
fn logo_layer(ids: Ids, logo: &Photo, width: f64, corner: &str) -> Layer {
    let height = width * logo.size.height / logo.size.width;
    let x = if corner == "bottom-left" { 86.0 } else { 1860.0 - width };
    let y = if corner == "top-right" { 60.0 } else { 1010.0 - height };
    photo(ids, &logo.url, x, y, width, height, None, "Logo")
}

pub const BAND: (f64, f64, f64, f64) = (82.0, 529.0, 1746.0, 371.0);
/// Where photos stand, inside the band.
pub const BASELINE: f64 = 775.0;
const ROW_WIDTH: f64 = 1500.0;
const LABEL_WIDTH: f64 = 320.0;
const NAME_SIZE: f64 = 22.0;
const CAPTION_SIZE: f64 = 15.0;
const LABEL_HEIGHT: f64 = NAME_SIZE * 1.15 + CAPTION_SIZE * 1.15;

#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub item: DeckItemInfo,
    pub url: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_box: bool,
}

/// `/\bbox/i` on "caption name".
fn is_box(item: &DeckItemInfo) -> bool {
    let s = format!("{} {}", item.caption, item.name).to_lowercase();
    s.match_indices("box")
        .any(|(i, _)| s.get(..i).and_then(|h| h.chars().last()).is_none_or(|c| !(c.is_alphanumeric() || c == '_')))
}

/// Scales a photo to a target height, capping its width so wide items don't dominate.
pub fn fit_photo(size: Size, target_height: f64, max_width: f64) -> (f64, f64) {
    let mut height = target_height;
    let mut width = size.width / size.height * height;
    if width > max_width {
        width = max_width;
        height = size.height / size.width * width;
    }
    (width, height)
}

/// Lays photos out in a row standing on the band: the gift box in the middle
/// and larger, overlapping its neighbours; everything else to either side.
// `windows(2)` always yields two items.
#[allow(clippy::indexing_slicing, clippy::type_complexity)]
pub fn arrange_row(items: &[(DeckItemInfo, String, Size)]) -> Vec<Placed> {
    if items.is_empty() {
        return Vec::new();
    }
    let box_index = items.iter().position(|(i, _, _)| is_box(i));
    let others: Vec<&(DeckItemInfo, String, Size)> =
        items.iter().enumerate().filter(|(i, _)| Some(*i) != box_index).map(|(_, x)| x).collect();
    let half = others.len().div_ceil(2);
    let mut ordered: Vec<(&(DeckItemInfo, String, Size), bool)> =
        others.iter().take(half).map(|x| (*x, false)).collect();
    if let Some(b) = box_index.and_then(|i| items.get(i)) {
        ordered.push((b, true));
    }
    ordered.extend(others.iter().skip(half).map(|x| (*x, false)));

    let sized: Vec<(&(DeckItemInfo, String, Size), bool, f64, f64)> = ordered
        .into_iter()
        .map(|(i, boxy)| {
            let (w, h) = fit_photo(i.2, if boxy { 460.0 } else { 360.0 }, if boxy { 560.0 } else { 300.0 });
            (i, boxy, w, h)
        })
        .collect();
    // Neighbours of the box tuck in front of it; other items get a small gap.
    let gaps: Vec<f64> =
        sized.windows(2).map(|w| if w[0].1 || w[1].1 { -0.14 * w[0].2.min(w[1].2) } else { 28.0 }).collect();
    let total = sized.iter().map(|s| s.2).sum::<f64>() + gaps.iter().sum::<f64>();
    let k = (ROW_WIDTH / total).min(1.0);
    let mut x = SLIDE_W / 2.0 - total * k / 2.0;
    sized
        .iter()
        .enumerate()
        .map(|(i, (it, boxy, w, h))| {
            let (width, height) = (w * k, h * k);
            let p =
                Placed { item: it.0.clone(), url: it.1.clone(), is_box: *boxy, x, y: BASELINE - height, width, height };
            x += width + gaps.get(i).copied().unwrap_or(0.0) * k;
            p
        })
        .collect()
}

fn overlaps(a: (f64, f64, f64, f64), b: &Placed) -> bool {
    a.0 < b.x + b.width && b.x < a.0 + a.2 && a.1 < b.y + b.height && b.y < a.1 + a.3
}

/// Name and caption beside each photo: left of items on the left, right of
/// the rest, lifted clear of a taller neighbour.
pub fn labels(ids: Ids, row: &[Placed]) -> Vec<Layer> {
    let pivot = row.iter().position(|p| p.is_box).unwrap_or(row.len() / 2);
    let mut out = Vec::new();
    for (i, p) in row.iter().enumerate() {
        let left = i < pivot;
        let mut b = (
            if left { p.x - 10.0 - LABEL_WIDTH } else { p.x + p.width + 10.0 },
            p.y + if p.is_box { 6.0 } else { 16.0 },
            LABEL_WIDTH,
            LABEL_HEIGHT,
        );
        let neighbour = if left { i.checked_sub(1).and_then(|j| row.get(j)) } else { row.get(i + 1) };
        if let Some(n) = neighbour
            && !p.is_box
            && overlaps(b, n)
        {
            b.1 = (n.y - LABEL_HEIGHT - 10.0).max(40.0);
        }
        let align = if left { Align::Right } else { Align::Left };
        let name = format!("{} label", p.item.name);
        out.push(text(
            ids,
            &p.item.name,
            b.0,
            b.1,
            TextOpts { size: NAME_SIZE, width: LABEL_WIDTH, align, name: &name, ..Default::default() },
        ));
        if !p.item.caption.is_empty() {
            let name = format!("{} caption", p.item.name);
            out.push(text(
                ids,
                &p.item.caption,
                b.0,
                b.1 + NAME_SIZE * 1.15,
                TextOpts { size: CAPTION_SIZE, width: LABEL_WIDTH, align, name: &name, ..Default::default() },
            ));
        }
    }
    out
}

pub fn cover_slide(ids: Ids, title: &str, subtitle: &str, logo: Option<&Photo>) -> Canvas {
    let mut layers = vec![
        text(
            ids,
            if title.is_empty() { "Premium" } else { title },
            61.0,
            64.0,
            TextOpts { size: 88.0, color: colors::HEADING, width: 1400.0, name: "Title", ..Default::default() },
        ),
        text(
            ids,
            subtitle,
            61.0,
            164.0,
            TextOpts { size: 38.0, color: colors::HEADING, width: 1400.0, name: "Subtitle", ..Default::default() },
        ),
    ];
    if let Some(l) = logo {
        layers.push(logo_layer(ids, l, 300.0, "bottom-right"));
    }
    page(colors::GREEN, layers)
}

pub fn closing_slide(ids: Ids, heading: &str, tagline: &str, contact: &str, logo: Option<&Photo>) -> Canvas {
    let mut layers = Vec::new();
    if let Some(l) = logo {
        layers.push(logo_layer(ids, l, 300.0, "top-right"));
    }
    let q =
        |size, width, name| TextOpts { font: "Questrial", size, color: "#111111", width, name, ..Default::default() };
    layers.push(text(
        ids,
        if heading.is_empty() { "Thank You" } else { heading },
        99.0,
        700.0,
        q(112.0, 900.0, "Thank you"),
    ));
    layers.push(text(ids, tagline, 104.0, 826.0, TextOpts { spacing: 4.0, ..q(48.0, 900.0, "Tagline") }));
    layers.push(text(
        ids,
        "For any Query",
        1022.0,
        690.0,
        TextOpts { align: Align::Right, underline: true, ..q(38.0, 800.0, "Query heading") },
    ));
    layers.push(text(
        ids,
        contact,
        822.0,
        752.0,
        TextOpts { align: Align::Right, spacing: 3.0, line_height: 1.3, ..q(34.0, 1000.0, "Contact") },
    ));
    page(colors::GREEN, layers)
}

fn item_page(
    ids: Ids,
    (title, subtitle): (String, String),
    price: Option<f64>,
    note: &str,
    logo: Option<&Photo>,
    middle: Vec<Layer>,
) -> Canvas {
    let mut layers = vec![
        text(
            ids,
            &title,
            86.0,
            64.0,
            TextOpts { size: 82.0, color: colors::HEADING, width: 1200.0, name: "Title", ..Default::default() },
        ),
        text(
            ids,
            &subtitle,
            86.0,
            158.0,
            TextOpts { size: 40.0, color: colors::HEADING, width: 1200.0, name: "Subtitle", ..Default::default() },
        ),
        rect(ids, BAND.0, BAND.1, BAND.2, BAND.3, colors::GREEN, "Band"),
    ];
    layers.extend(middle);
    if let Some(p) = price {
        layers.push(text(
            ids,
            &format!("Price : {} + Tax", format_inr(p)),
            1213.0,
            88.0,
            TextOpts {
                size: 18.0,
                style: FontStyle::Bold,
                color: colors::HEADING,
                width: 600.0,
                align: Align::Right,
                name: "Price",
                ..Default::default()
            },
        ));
        layers.push(text(
            ids,
            "Shipping & Packaging\nWill Cost you Extra",
            1213.0,
            112.0,
            TextOpts {
                size: 14.0,
                color: colors::MUTED,
                width: 600.0,
                align: Align::Right,
                name: "Price note",
                ..Default::default()
            },
        ));
    }
    if let Some(l) = logo {
        layers.push(logo_layer(ids, l, 190.0, "bottom-left"));
    }
    if !note.trim().is_empty() {
        layers.push(text(
            ids,
            note,
            1128.0,
            972.0,
            TextOpts {
                size: 17.0,
                color: colors::MUTED,
                width: 700.0,
                align: Align::Right,
                line_height: 1.3,
                name: "Terms",
                ..Default::default()
            },
        ));
    }
    page(colors::PAGE, layers)
}

fn band_message(ids: Ids, message: &str) -> Layer {
    text(
        ids,
        message,
        BAND.0,
        BAND.1 + BAND.3 / 2.0 - 16.0,
        TextOpts {
            size: 28.0,
            color: colors::HEADING,
            width: BAND.2,
            align: Align::Center,
            name: "Placeholder",
            ..Default::default()
        },
    )
}

/// Looks up a loaded photo's natural size; None if it didn't load.
pub type SizeOf<'a> = &'a dyn Fn(&str) -> Option<Size>;

pub fn hamper_slide(ids: Ids, h: &HamperInfo, size_of: SizeOf, note: &str, logo: Option<&Photo>) -> Canvas {
    let mut seen: Vec<String> = Vec::new();
    let with_photos: Vec<(DeckItemInfo, String, Size)> = h
        .items
        .iter()
        .filter_map(|item| {
            let url = item.image_url.clone()?;
            let size = size_of(&url)?;
            let key = item.product_id.clone().unwrap_or_else(|| item.name.clone());
            if seen.contains(&key) {
                return None;
            }
            seen.push(key);
            Some((item.clone(), url, size))
        })
        .collect();
    let middle = if !with_photos.is_empty() {
        let row = arrange_row(&with_photos);
        // The box is drawn first so the other items sit in front of it.
        let mut by_depth = row.clone();
        by_depth.sort_by_key(|p| !p.is_box);
        let mut m: Vec<Layer> = by_depth
            .iter()
            .map(|p| photo(ids, &p.url, p.x, p.y, p.width, p.height, p.item.product_id.clone(), &p.item.name))
            .collect();
        m.extend(labels(ids, &row));
        m
    } else if let Some((url, size)) = h.image_url.as_ref().and_then(|u| size_of(u).map(|s| (u, s))) {
        let (w, hh) = fit_photo(size, 560.0, 1100.0);
        vec![photo(ids, url, SLIDE_W / 2.0 - w / 2.0, BASELINE - hh, w, hh, None, "Hamper image")]
    } else {
        vec![band_message(ids, "Add product photos to show this hamper's contents")]
    };
    item_page(ids, split_title(&h.name, "Hamper"), h.price, note, logo, middle)
}

pub fn product_slide(ids: Ids, p: &ProductInfo, size_of: SizeOf, note: &str, logo: Option<&Photo>) -> Canvas {
    let middle = match p.image_url.as_ref().and_then(|u| size_of(u).map(|s| (u, s))) {
        Some((url, size)) => {
            let (w, h) = fit_photo(size, 520.0, 900.0);
            let placed = Placed {
                item: DeckItemInfo {
                    product_id: Some(p.id.clone()),
                    name: p.name.clone(),
                    caption: p.caption.clone(),
                    image_url: Some(url.clone()),
                },
                url: url.clone(),
                is_box: false,
                x: SLIDE_W / 2.0 - w / 2.0,
                y: BASELINE - h,
                width: w,
                height: h,
            };
            let mut m = vec![photo(ids, url, placed.x, placed.y, w, h, Some(p.id.clone()), &p.name)];
            m.extend(labels(ids, &[placed]));
            m
        }
        None => vec![band_message(ids, "Add a photo to this product to show it here")],
    };
    item_page(ids, (p.name.clone(), p.caption.clone()), p.price, note, logo, middle)
}

pub fn blank_slide() -> Canvas {
    page(colors::PAGE, Vec::new())
}

/// PowerPoint measures in inches; slides use 144 px per inch (1920 px = 13.33").
pub const PX_PER_INCH: f64 = 144.0;

/// A text layer as a native PowerPoint text box, in inches and points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PptTextBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub rotate: f64,
    pub font_face: String,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// RRGGBB.
    pub color: String,
    pub align: Align,
    pub char_spacing: f64,
    pub line_spacing_multiple: f64,
    pub transparency: f64,
}

/// Konva rotates a box about its top-left corner, PowerPoint about its centre,
/// so the box goes where Konva's rotated centre lands. `height` is the laid-out
/// text height in px.
pub fn ppt_text_box(base: &Base, t: &Text, height: f64) -> PptTextBox {
    let (sx, sy) = (base.scale_x.abs(), base.scale_y.abs());
    let (w, h) = (t.width * sx, height * sy);
    let rad = base.rotation * std::f64::consts::PI / 180.0;
    let cx = base.x + w / 2.0 * rad.cos() - h / 2.0 * rad.sin();
    let cy = base.y + w / 2.0 * rad.sin() + h / 2.0 * rad.cos();
    let inch = |px: f64| px / PX_PER_INCH;
    let pt = |px: f64| px * 72.0 / PX_PER_INCH;
    PptTextBox {
        x: inch(cx - w / 2.0),
        y: inch(cy - h / 2.0),
        w: inch(w),
        h: inch(h),
        rotate: (base.rotation % 360.0 + 360.0) % 360.0,
        font_face: t.font_family.clone(),
        font_size: crate::js::round(pt(t.font_size * sy) * 10.0) / 10.0,
        bold: t.font_style.bold(),
        italic: t.font_style.italic(),
        underline: t.text_decoration == Decoration::Underline,
        color: t.fill.main_color().trim_start_matches('#').to_uppercase(),
        align: t.align,
        char_spacing: pt(t.letter_spacing * sx),
        line_spacing_multiple: t.line_height,
        transparency: crate::js::round((1.0 - base.opacity) * 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("id{n}")
        }
    }

    fn item(name: &str, caption: &str, url: Option<&str>) -> DeckItemInfo {
        DeckItemInfo { product_id: None, name: name.into(), caption: caption.into(), image_url: url.map(String::from) }
    }

    fn sizes(u: &str) -> Option<Size> {
        let (w, h) = match u {
            "https://e.com/box.png" => (1200.0, 1000.0),
            "https://e.com/mug.png" => (900.0, 1000.0),
            "https://e.com/book.png" => (750.0, 1000.0),
            "https://e.com/card.png" => (1470.0, 1000.0),
            "https://e.com/logo.png" => (1600.0, 887.0),
            _ => return None,
        };
        Some(Size { width: w, height: h })
    }

    #[test]
    fn slides_are_valid_with_unique_ids() {
        let mut next = ids();
        let logo = Photo { url: "https://e.com/logo.png".into(), size: Size { width: 1600.0, height: 887.0 } };
        let h = HamperInfo {
            id: "h".into(),
            code: "H1".into(),
            name: "Essential Hamper".into(),
            price: Some(3250.0),
            image_url: None,
            items: vec![
                item("Card Holder", "Genuine Leather", Some("https://e.com/card.png")),
                item("A5 Note Book", "Custom", Some("https://e.com/book.png")),
                item("Wooden Lift Top Box", "Box & Packaging", Some("https://e.com/box.png")),
                item("Cork Base Coffee Mug", "Drinkware", Some("https://e.com/mug.png")),
                item("No photo item", "Edibles", None),
            ],
        };
        let bare = HamperInfo {
            id: "h2".into(),
            code: "H2".into(),
            name: "Bare".into(),
            price: None,
            image_url: None,
            items: vec![],
        };
        let p = ProductInfo {
            id: "p".into(),
            code: "P1".into(),
            name: "Mug".into(),
            caption: "Drinkware".into(),
            price: Some(900.0),
            image_url: Some("https://e.com/mug.png".into()),
        };
        let slides = [
            cover_slide(&mut next, "Premium", "Giveaway Hamper", Some(&logo)),
            hamper_slide(&mut next, &h, &sizes, "MOQ note", Some(&logo)),
            hamper_slide(&mut next, &bare, &sizes, "", None),
            product_slide(&mut next, &p, &sizes, "", Some(&logo)),
            closing_slide(&mut next, "Thank You", "for your time", "+91 81975 44505", Some(&logo)),
        ];
        for s in &slides {
            assert!(s.valid());
        }
        let mut all: Vec<&str> = slides.iter().flat_map(|s| s.layers.iter().map(Layer::id)).collect();
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n);
        let hs = &slides[1];
        let has_text = |c: &Canvas, s: &str| c.layers.iter().any(|l| matches!(&l.shape, Shape::Text(t) if t.text == s));
        assert!(has_text(hs, "Price : INR 3,250 + Tax") && has_text(hs, "Essential"));
        let photos: Vec<&Layer> =
            hs.layers.iter().filter(|l| l.kind() == "image" && l.base.name.as_deref() != Some("Logo")).collect();
        assert_eq!(photos.len(), 4);
        assert_eq!(photos[0].base.name.as_deref(), Some("Wooden Lift Top Box"));
        assert!(slides[2].layers.iter().any(|l| l.base.name.as_deref() == Some("Placeholder")));
        assert!(!slides[2].layers.iter().any(|l| l.base.name.as_deref() == Some("Logo")));
    }

    #[test]
    fn row_layout() {
        let s = |u| sizes(u).unwrap_or(Size { width: 1.0, height: 1.0 });
        let row = arrange_row(&[
            (item("Card", "Leather", None), "c".into(), s("https://e.com/card.png")),
            (item("Book", "Custom", None), "b".into(), s("https://e.com/book.png")),
            (item("Box", "Box & Packaging", None), "x".into(), s("https://e.com/box.png")),
            (item("Mug", "Drinkware", None), "m".into(), s("https://e.com/mug.png")),
        ]);
        assert_eq!(row.iter().map(|p| p.item.name.as_str()).collect::<Vec<_>>(), ["Card", "Book", "Box", "Mug"]);
        for p in &row {
            assert!((p.y + p.height - 775.0).abs() < 1e-9);
        }
        let (l, r) = (row[0].x, row[3].x + row[3].width);
        assert!(r - l <= 1500.0 + 1e-9 && ((l + r) / 2.0 - 960.0).abs() < 1e-9);
    }

    #[test]
    fn wording() {
        assert_eq!(split_title("Essential Hamper", "Hamper"), ("Essential".into(), "Hamper".into()));
        assert_eq!(split_title("Deskmate", "Hamper"), ("Deskmate".into(), "Hamper".into()));
        assert_eq!(split_title("Diwali hampers", "Hamper"), ("Diwali".into(), "Hampers".into()));
        assert_eq!(format_inr(3250.0), "INR 3,250");
        assert_eq!(format_inr(125000.4), "INR 1,25,000");
    }

    #[test]
    fn ppt_geometry() {
        let base = Base { opacity: 0.75, ..Base::at("t".into(), 144.0, 288.0) };
        let t = Text {
            text: "Hi".into(),
            font_size: 48.0,
            font_family: "Inter".into(),
            font_style: FontStyle::Bold,
            align: Align::Center,
            width: 288.0,
            letter_spacing: 4.0,
            line_height: 1.2,
            text_decoration: Decoration::Underline,
            fill: Fill::solid("#54655b"),
        };
        let b = ppt_text_box(&base, &t, 144.0);
        assert_eq!((b.x, b.y, b.w, b.h, b.font_size, b.char_spacing), (1.0, 2.0, 2.0, 1.0, 24.0, 2.0));
        assert_eq!((b.color.as_str(), b.transparency, b.underline), ("54655B", 25.0, true));
        let r = ppt_text_box(&Base { rotation: 90.0, ..base }, &t, 144.0);
        assert!((r.x - (144.0 - 72.0 - 144.0) / 144.0).abs() < 1e-9);
        assert!((r.y - (288.0 + 144.0 - 72.0) / 144.0).abs() < 1e-9);
    }
}
