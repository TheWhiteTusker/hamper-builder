//! Magic layers' pixel work (port of `lib/magic-layers/*`): choosing the
//! object masks the segmentation model found, softening their edges at full
//! resolution, filling the holes left behind, reading a text line's colour
//! and weight, and placing the pieces back where they were on the page.
//!
//! The models only guide where to cut; every pixel comes from the original.

use super::{Base, Fit, Image};
use serde::{Deserialize, Serialize};

/// One decoder mask: w×h logits (> 0 is inside) and the model's quality estimate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LowMask {
    pub logits: Vec<f32>,
    pub w: usize,
    pub h: usize,
    pub score: f32,
}

/// A rectangle in pixels (or mask cells).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// A text line's box in source pixels, as OCR reports it (x1, y1 exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineBox {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

fn bits(m: &LowMask) -> Vec<bool> {
    m.logits.iter().map(|v| *v > 0.0).collect()
}

fn overlap(a: &[bool], b: &[bool]) -> usize {
    a.iter().zip(b).filter(|(x, y)| **x && **y).count()
}

#[derive(Clone, Copy, Debug)]
pub struct PickOpts {
    pub min_score: f32,
    pub min_area: f64,
    pub max_area: f64,
    pub max_iou: f64,
    pub contained: f64,
}

impl Default for PickOpts {
    fn default() -> Self {
        Self { min_score: 0.88, min_area: 0.003, max_area: 0.4, max_iou: 0.7, contained: 0.85 }
    }
}

/// The masks worth turning into layers: confident, neither specks nor the
/// whole backdrop, no near-duplicates, and no pieces of an object already
/// kept (a bottle's label goes with the bottle). Best first.
pub fn pick_objects(masks: &[LowMask], o: PickOpts) -> Vec<LowMask> {
    let mut scored: Vec<(&LowMask, Vec<bool>, usize)> = masks
        .iter()
        .map(|m| {
            let b = bits(m);
            let area = b.iter().filter(|x| **x).count();
            (m, b, area)
        })
        .filter(|(m, _, area)| {
            let cells = (m.w * m.h) as f64;
            m.score >= o.min_score && *area as f64 >= o.min_area * cells && *area as f64 <= o.max_area * cells
        })
        .collect();
    scored.sort_by(|a, b| b.0.score.total_cmp(&a.0.score));
    let mut kept: Vec<(&LowMask, Vec<bool>, usize)> = Vec::new();
    for c in scored {
        let dupe = kept.iter().any(|k| {
            let i = overlap(&c.1, &k.1) as f64;
            i / ((c.2 + k.2) as f64 - i) > o.max_iou
        });
        if !dupe {
            kept.push(c);
        }
    }
    kept.iter()
        .enumerate()
        .filter(|(ci, c)| !kept.iter().enumerate().any(|(ki, k)| ki != *ci && k.2 > c.2 && overlap(&c.1, &k.1) as f64 / c.2 as f64 >= o.contained))
        .map(|(_, c)| c.0.clone())
        .collect()
}

/// Share of the mask inside any of `boxes` (mask cells): text OCR found
/// shouldn't also become an object.
pub fn share_inside(m: &LowMask, boxes: &[(f64, f64, f64, f64)]) -> f64 {
    let b = bits(m);
    let (mut total, mut inside) = (0usize, 0usize);
    for y in 0..m.h {
        for x in 0..m.w {
            if !b.get(y * m.w + x).copied().unwrap_or(false) {
                continue;
            }
            total += 1;
            let (fx, fy) = (x as f64, y as f64);
            if boxes.iter().any(|(rx, ry, rw, rh)| fx >= *rx && fx < rx + rw && fy >= *ry && fy < ry + rh) {
                inside += 1;
            }
        }
    }
    if total == 0 { 0.0 } else { inside as f64 / total as f64 }
}

/// The mask's bounds in source pixels, padded, clamped to the image. `scale`
/// is mask cells per source pixel.
pub fn source_box(m: &LowMask, scale: f64, src_w: usize, src_h: usize, pad: usize) -> Option<Rect> {
    let b = bits(m);
    let (mut x0, mut y0, mut x1, mut y1) = (m.w, m.h, None::<usize>, 0usize);
    for y in 0..m.h {
        for x in 0..m.w {
            if !b.get(y * m.w + x).copied().unwrap_or(false) {
                continue;
            }
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = Some(x1.map_or(x, |v| v.max(x)));
            y1 = y1.max(y);
        }
    }
    let x1 = x1?;
    let left = ((x0 as f64 / scale).floor() as usize).saturating_sub(pad);
    let top = ((y0 as f64 / scale).floor() as usize).saturating_sub(pad);
    let right = (((x1 + 1) as f64 / scale).ceil() as usize + pad).min(src_w);
    let bottom = (((y1 + 1) as f64 / scale).ceil() as usize + pad).min(src_h);
    Some(Rect { x: left, y: top, w: right - left, h: bottom - top })
}

/// Alpha (0–255) for every source pixel in `r`: the logits sampled
/// bilinearly at full resolution through a sigmoid, so edges are soft.
pub fn alpha_for(m: &LowMask, scale: f64, r: Rect) -> Vec<u8> {
    let at = |x: i64, y: i64| {
        let (cx, cy) = (x.clamp(0, m.w as i64 - 1) as usize, y.clamp(0, m.h as i64 - 1) as usize);
        f64::from(m.logits.get(cy * m.w + cx).copied().unwrap_or(-8.0))
    };
    let mut out = Vec::with_capacity(r.w * r.h);
    for j in 0..r.h {
        let v = ((r.y + j) as f64 + 0.5) * scale - 0.5;
        let y0 = v.floor();
        let fy = v - y0;
        for i in 0..r.w {
            let u = ((r.x + i) as f64 + 0.5) * scale - 0.5;
            let x0 = u.floor();
            let fx = u - x0;
            let (xi, yi) = (x0 as i64, y0 as i64);
            let top = at(xi, yi) * (1.0 - fx) + at(xi + 1, yi) * fx;
            let bottom = at(xi, yi + 1) * (1.0 - fx) + at(xi + 1, yi + 1) * fx;
            let a = 255.0 / (1.0 + (-(top * (1.0 - fy) + bottom * fy)).exp());
            // Uint8ClampedArray rounds on store.
            out.push(a.round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

/// Fills holes ("onion peel"): every hole pixel touching known pixels takes
/// their average, ring by ring.
///
/// ponytail: averaging fill. Clean on flat or gradient backdrops, smeary in
/// big holes on textured photos; an inpainting model is the upgrade.
// Indices stay within `0..w * h`; buffer sizes are checked on entry.
#[allow(clippy::indexing_slicing)]
pub fn fill_holes(rgba: &mut [u8], w: usize, h: usize, hole: &[bool]) {
    let n = w * h;
    if rgba.len() < n * 4 || hole.len() < n {
        return;
    }
    let mut known: Vec<bool> = hole.iter().take(n).map(|x| !x).collect();
    let mut queued = vec![false; n];
    let around = |i: usize| {
        let (x, y) = ((i % w) as i64, (i / w) as i64);
        (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy))).filter(|(nx, ny)| *nx >= 0 && *ny >= 0 && *nx < w as i64 && *ny < h as i64).map(|(nx, ny)| ny as usize * w + nx as usize)
    };
    let mut ring: Vec<usize> = Vec::new();
    for i in 0..n {
        if known[i] {
            for nb in around(i) {
                if !known[nb] && !queued[nb] {
                    queued[nb] = true;
                    ring.push(nb);
                }
            }
        }
    }
    while !ring.is_empty() {
        let current = std::mem::take(&mut ring);
        let values: Vec<[u8; 4]> = current
            .iter()
            .map(|&i| {
                let (mut sums, mut cnt) = ([0f64; 4], 0f64);
                for nb in around(i) {
                    if known[nb] {
                        for (c, s) in sums.iter_mut().enumerate() {
                            *s += f64::from(rgba[nb * 4 + c]);
                        }
                        cnt += 1.0;
                    }
                }
                // Uint8ClampedArray rounds half to even; plain rounding is within a unit.
                sums.map(|s| (s / cnt.max(1.0)).round().clamp(0.0, 255.0) as u8)
            })
            .collect();
        // Commit the whole ring at once: each ring borrows only from the ones outside it.
        for (k, &i) in current.iter().enumerate() {
            rgba[i * 4..i * 4 + 4].copy_from_slice(&values[k]);
            known[i] = true;
        }
        for &i in &current {
            for nb in around(i) {
                if !known[nb] && !queued[nb] {
                    queued[nb] = true;
                    ring.push(nb);
                }
            }
        }
    }
}

/// Grows a mask by `r` pixels in every direction (square brush).
// Indices stay within `0..w * h` by construction.
#[allow(clippy::indexing_slicing)]
pub fn dilate(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    if r == 0 {
        return mask.to_vec();
    }
    let mut across = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if mask.get(y * w + x).copied().unwrap_or(false) {
                for nx in x.saturating_sub(r)..=(x + r).min(w - 1) {
                    across[y * w + nx] = true;
                }
            }
        }
    }
    let mut out = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if across[y * w + x] {
                for ny in y.saturating_sub(r)..=(y + r).min(h - 1) {
                    out[ny * w + x] = true;
                }
            }
        }
    }
    out
}

/// The colour (#rrggbb) and weight of the text inside `b`. The box's edge is
/// mostly backdrop, so pixels far from the edge colour are ink.
///
/// ponytail: "bold" is ink covering over 30% of a tight line box.
pub fn text_style(rgba: &[u8], w: usize, b: LineBox) -> (String, bool) {
    let px = |x: usize, y: usize| -> [f64; 3] {
        let p = (y * w + x) * 4;
        [0, 1, 2].map(|c| f64::from(rgba.get(p + c).copied().unwrap_or(0)))
    };
    let (mut edge, mut edges) = ([0f64; 3], 0f64);
    let mut add = |v: [f64; 3]| {
        for (e, x) in edge.iter_mut().zip(v) {
            *e += x;
        }
        edges += 1.0;
    };
    for x in b.x0..b.x1 {
        add(px(x, b.y0));
        add(px(x, b.y1.saturating_sub(1)));
    }
    for y in (b.y0 + 1)..b.y1.saturating_sub(1) {
        add(px(b.x0, y));
        add(px(b.x1.saturating_sub(1), y));
    }
    let bg = edge.map(|v| v / edges.max(1.0));
    let mut dist = Vec::new();
    let mut max = 0f64;
    for y in b.y0..b.y1 {
        for x in b.x0..b.x1 {
            let p = px(x, y);
            let d = ((p[0] - bg[0]).powi(2) + (p[1] - bg[1]).powi(2) + (p[2] - bg[2]).powi(2)).sqrt();
            max = max.max(d);
            dist.push(d);
        }
    }
    if max < 48.0 {
        return ("#000000".into(), false);
    }
    let (mut ink, mut clear, mut inked, mut i) = ([0f64; 3], 0f64, 0usize, 0usize);
    for y in b.y0..b.y1 {
        for x in b.x0..b.x1 {
            let d = dist.get(i).copied().unwrap_or(0.0);
            i += 1;
            if d >= 48.0 {
                inked += 1;
            }
            if d < max * 0.6 {
                continue;
            }
            let p = px(x, y);
            for (k, x) in ink.iter_mut().zip(p) {
                *k += x;
            }
            clear += 1.0;
        }
    }
    let hex: String = ink.iter().map(|v| format!("{:02x}", crate::js::round(v / clear) as u8)).collect();
    (format!("#{hex}"), inked as f64 / dist.len().max(1) as f64 > 0.3)
}

/// Maps source-pixel positions in a split image onto the page, so pieces land
/// exactly where they were drawn in the original layer, whatever its fit,
/// scale, flip or rotation. Sizes are in the parent's frame units (× kx, ky).
#[derive(Clone, Debug, PartialEq)]
pub struct FramePlacer {
    pub kx: f64,
    pub ky: f64,
    ox: f64,
    oy: f64,
    base: Base,
}

impl FramePlacer {
    pub fn new(base: &Base, i: &Image, nat_w: f64, nat_h: f64) -> Self {
        let (mut kx, mut ky, mut ox, mut oy) = (i.width / nat_w, i.height / nat_h, 0.0, 0.0);
        if i.fit == Fit::Contain {
            kx = kx.min(ky);
            ky = kx;
            ox = (i.width - nat_w * kx) / 2.0;
            oy = (i.height - nat_h * ky) / 2.0;
        }
        Self { kx, ky, ox, oy, base: base.clone() }
    }

    /// Page position of source pixel (px, py).
    pub fn at(&self, px: f64, py: f64) -> (f64, f64) {
        let b = &self.base;
        let rad = b.rotation.to_radians();
        let lx = (self.ox + px * self.kx) * b.scale_x;
        let ly = (self.oy + py * self.ky) * b.scale_y;
        (b.x + lx * rad.cos() - ly * rad.sin(), b.y + lx * rad.sin() + ly * rad.cos())
    }

    /// A child layer's base at source pixel (px, py), sharing the parent's transform.
    pub fn base_at(&self, id: String, px: f64, py: f64, name: &str) -> Base {
        let (x, y) = self.at(px, py);
        Base {
            id,
            name: Some(name.into()),
            visible: true,
            locked: false,
            x,
            y,
            rotation: self.base.rotation,
            scale_x: self.base.scale_x,
            scale_y: self.base.scale_y,
            opacity: self.base.opacity,
        }
    }
}

/// A font size that makes the line as wide as the box OCR found, starting
/// from the row height and staying within ±30% of it.
pub fn fit_font_size(row_height: f64, box_width: f64, width_at_100: f64) -> f64 {
    if width_at_100 <= 0.0 {
        return row_height;
    }
    (100.0 * box_width / width_at_100).clamp(row_height * 0.7, row_height * 1.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(rows: &[&str], score: f32) -> LowMask {
        let logits = rows.concat().chars().map(|c| if c == '#' { 8.0 } else { -8.0 }).collect();
        LowMask { logits, w: rows[0].len(), h: rows.len(), score }
    }

    fn image(rows: &[&str]) -> (Vec<u8>, usize, usize) {
        let data = rows
            .concat()
            .chars()
            .flat_map(|c| match c {
                'K' => [0, 0, 0, 255],
                'R' => [200, 0, 0, 255],
                _ => [255, 255, 255, 255],
            })
            .collect();
        (data, rows[0].len(), rows.len())
    }

    fn big() -> LowMask {
        mask(&["......", ".####.", ".####.", ".####.", ".####.", "......"], 0.95)
    }

    #[test]
    fn picks_objects() {
        let part = mask(&["......", ".##...", ".##...", "......", "......", "......"], 0.99);
        let twin = mask(&["......", ".####.", ".####.", ".####.", ".###..", "......"], 0.9);
        let backdrop = mask(&["######", "######", "######", "######", "######", "#####."], 0.95);
        let speck = mask(&["......", "......", "..#...", "......", "......", "......"], 0.95);
        let shy = mask(&["####..", "####..", "......", "......", "......", "......"], 0.5);
        let o = PickOpts { min_area: 0.05, max_area: 0.6, ..Default::default() };
        assert_eq!(pick_objects(&[part, twin, big(), backdrop, speck, shy], o), vec![big()]);
        let left = mask(&["##....", "##....", "......", "......", "......", "......"], 0.9);
        let right = mask(&["......", "......", "......", "......", "....##", "....##"], 0.97);
        assert_eq!(pick_objects(&[left.clone(), right.clone()], PickOpts { min_area: 0.05, ..Default::default() }), vec![right, left]);
    }

    #[test]
    fn boxes_and_shares() {
        assert_eq!(share_inside(&big(), &[(1.0, 1.0, 4.0, 2.0)]), 0.5);
        assert_eq!(share_inside(&big(), &[]), 0.0);
        assert_eq!(source_box(&big(), 0.1, 60, 60, 2), Some(Rect { x: 8, y: 8, w: 44, h: 44 }));
        assert_eq!(source_box(&mask(&["..", ".."], 0.9), 1.0, 2, 2, 2), None);
    }

    #[test]
    fn soft_alpha() {
        let a = alpha_for(&big(), 0.1, Rect { x: 0, y: 30, w: 60, h: 1 });
        assert_eq!((a[30], a[0]), (255, 0));
        let edge = &a[5..16];
        assert!(edge.iter().any(|v| *v > 0 && *v < 255));
        assert!(edge.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn fills_and_dilates() {
        let (mut d, w, h) = image(&["RRRRR", "RWWWR", "RWWWR", "RRRRR"]);
        let hole: Vec<bool> = "00000011100111000000".chars().map(|c| c == '1').collect();
        fill_holes(&mut d, w, h, &hole);
        assert!(d.chunks(4).all(|p| p == [200, 0, 0, 255]));
        let (mut d, w, h) = image(&["KWR"]);
        fill_holes(&mut d, w, h, &[false, true, false]);
        assert_eq!(&d[4..8], &[100, 0, 0, 255]);
        let mut m = vec![false; 25];
        m[12] = true;
        let s: String = dilate(&m, 5, 5, 1).iter().map(|b| if *b { '1' } else { '0' }).collect();
        assert_eq!(s, "0000001110011100111000000");
    }

    #[test]
    fn text_colour_and_weight() {
        let (thin, w, _) = image(&["WWWWWWWW", "WKWWWWKW", "WKWWWWKW", "WWWWWWWW"]);
        assert_eq!(text_style(&thin, w, LineBox { x0: 0, y0: 0, x1: 8, y1: 4 }), ("#000000".into(), false));
        let (heavy, w, _) = image(&["WWWWWWWW", "WRRRRRRW", "WRRRRRRW", "WWWWWWWW"]);
        assert_eq!(text_style(&heavy, w, LineBox { x0: 0, y0: 0, x1: 8, y1: 4 }), ("#c80000".into(), true));
    }

    #[test]
    fn placing_pieces() {
        let base = Base::at("p".into(), 100.0, 50.0);
        let img = Image { product_id: None, url: "https://x.test/a.png".into(), width: 200.0, height: 100.0, fit: Fit::Stretch };
        let p = FramePlacer::new(&base, &img, 400.0, 400.0);
        assert_eq!((p.kx, p.ky, p.at(200.0, 200.0)), (0.5, 0.25, (200.0, 100.0)));
        let c = FramePlacer::new(&base, &Image { fit: Fit::Contain, ..img.clone() }, 400.0, 400.0);
        assert_eq!((c.kx, c.at(0.0, 0.0)), (0.25, (150.0, 50.0)));
        let turned = FramePlacer::new(&Base { rotation: 90.0, scale_x: 2.0, ..base }, &img, 200.0, 100.0);
        let (a, b) = (turned.at(10.0, 0.0), turned.at(0.0, 10.0));
        assert!((a.0 - 100.0).abs() < 1e-9 && (a.1 - 70.0).abs() < 1e-9);
        assert!((b.0 - 90.0).abs() < 1e-9 && (b.1 - 50.0).abs() < 1e-9);
        assert_eq!(fit_font_size(40.0, 200.0, 500.0), 40.0);
        assert_eq!(fit_font_size(40.0, 220.0, 500.0), 44.0);
        assert_eq!(fit_font_size(40.0, 1000.0, 500.0), 52.0);
        assert_eq!(fit_font_size(40.0, 10.0, 500.0), 28.0);
    }
}
