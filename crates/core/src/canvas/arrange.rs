//! Layer order, snapping, aligning and distributing (port of
//! `canvas-arrange.ts` and `canvas-snap-angle.ts`).

use super::Layer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerMove {
    Front,
    Forward,
    Backward,
    Back,
}

/// Arranges several layers at once, keeping their order relative to each
/// other. Forward/backward step each past its nearest unselected neighbour.
/// Returns the layers unchanged when nothing moves.
pub fn move_layers<T: Clone>(layers: &[T], id_of: impl Fn(&T) -> &str, ids: &[String], m: LayerMove) -> Vec<T> {
    let pick = |l: &T| ids.iter().any(|i| i == id_of(l));
    if !layers.iter().any(pick) {
        return layers.to_vec();
    }
    let (chosen, rest): (Vec<T>, Vec<T>) = layers.iter().cloned().partition(|l| pick(l));
    match m {
        LayerMove::Front => rest.into_iter().chain(chosen).collect(),
        LayerMove::Back => chosen.into_iter().chain(rest).collect(),
        LayerMove::Forward => {
            let mut next = layers.to_vec();
            for i in (0..next.len().saturating_sub(1)).rev() {
                if next.get(i).is_some_and(pick) && !next.get(i + 1).is_some_and(pick) {
                    next.swap(i, i + 1);
                }
            }
            next
        }
        LayerMove::Backward => {
            let mut next = layers.to_vec();
            for i in 1..next.len() {
                if next.get(i).is_some_and(pick) && !next.get(i - 1).is_some_and(pick) {
                    next.swap(i, i - 1);
                }
            }
            next
        }
    }
}

pub fn move_canvas_layers(layers: &[Layer], ids: &[String], m: LayerMove) -> Vec<Layer> {
    move_layers(layers, |l: &Layer| l.id(), ids, m)
}

/// Moves a layer to array index `to` (drag-and-drop in the Layers panel).
pub fn reorder_layer<T: Clone>(layers: &[T], from: usize, to: usize) -> Vec<T> {
    let mut next = layers.to_vec();
    if from >= next.len() || next.is_empty() {
        return next;
    }
    let target = to.min(next.len() - 1);
    if from != target {
        let l = next.remove(from);
        next.insert(target, l);
    }
    next
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Box {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snap {
    pub dx: f64,
    pub dy: f64,
    pub vertical: Vec<f64>,
    pub horizontal: Vec<f64>,
}

/// Left/centre/right (or top/middle/bottom) lines of a box on one axis.
fn stops(start: f64, size: f64) -> [f64; 3] {
    [start, start + size / 2.0, start + size]
}

/// How far to nudge a dragged box so its nearest edge or centre lines up with
/// the page or another item, per axis, plus the guides to draw. An axis with
/// nothing within `threshold` is left alone.
pub fn snap(b: Box, others: &[Box], page_w: f64, page_h: f64, threshold: f64) -> Snap {
    let axis = |own: [f64; 3], targets: Vec<f64>| {
        let mut best: Option<(f64, f64)> = None;
        for t in targets {
            for o in own {
                let delta = t - o;
                if delta.abs() <= threshold && best.is_none_or(|(d, _)| delta.abs() < d.abs()) {
                    best = Some((delta, t));
                }
            }
        }
        best
    };
    let v = axis(stops(b.x, b.width), stops(0.0, page_w).into_iter().chain(others.iter().flat_map(|o| stops(o.x, o.width))).collect());
    let h = axis(stops(b.y, b.height), stops(0.0, page_h).into_iter().chain(others.iter().flat_map(|o| stops(o.y, o.height))).collect());
    Snap {
        dx: v.map_or(0.0, |s| s.0),
        dy: h.map_or(0.0, |s| s.0),
        vertical: v.map(|s| s.1).into_iter().collect(),
        horizontal: h.map(|s| s.1).into_iter().collect(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageAlign {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

/// Offset that aligns a box to an edge or centre of the page.
pub fn align_to_page(b: Box, page_w: f64, page_h: f64, at: PageAlign) -> (f64, f64) {
    match at {
        PageAlign::Left => (-b.x, 0.0),
        PageAlign::Center => ((page_w - b.width) / 2.0 - b.x, 0.0),
        PageAlign::Right => (page_w - b.width - b.x, 0.0),
        PageAlign::Top => (0.0, -b.y),
        PageAlign::Middle => (0.0, (page_h - b.height) / 2.0 - b.y),
        PageAlign::Bottom => (0.0, page_h - b.height - b.y),
    }
}

/// Uniform scale that makes a box fit inside the page (never enlarges).
pub fn fit_scale(b: Box, page_w: f64, page_h: f64, margin: f64) -> f64 {
    1f64.min(page_w * margin / b.width).min(page_h * margin / b.height)
}

/// The smallest box containing all the boxes.
pub fn bounds_of(boxes: &[Box]) -> Box {
    let x = boxes.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    let y = boxes.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let right = boxes.iter().map(|b| b.x + b.width).fold(f64::NEG_INFINITY, f64::max);
    let bottom = boxes.iter().map(|b| b.y + b.height).fold(f64::NEG_INFINITY, f64::max);
    Box { x, y, width: right - x, height: bottom - y }
}

pub fn intersects(a: Box, b: Box) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// Offsets that align each box to the shared edge or centre of the selection.
pub fn align_boxes(boxes: &[Box], at: PageAlign) -> Vec<(f64, f64)> {
    let b = bounds_of(boxes);
    boxes
        .iter()
        .map(|bx| {
            let (dx, dy) = align_to_page(Box { x: bx.x - b.x, y: bx.y - b.y, ..*bx }, b.width, b.height, at);
            (dx + 0.0, dy + 0.0)
        })
        .collect()
}

/// Offsets that space boxes evenly between the outermost two, which stay
/// put. Order follows position on that axis, not selection order.
pub fn distribute_boxes(boxes: &[Box], horizontal: bool) -> Vec<(f64, f64)> {
    let mut out = vec![(0.0, 0.0); boxes.len()];
    if boxes.len() < 3 {
        return out;
    }
    let pos = |b: &Box| if horizontal { b.x } else { b.y };
    let size = |b: &Box| if horizontal { b.width } else { b.height };
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&i, &j| pos(&boxes[i]).total_cmp(&pos(&boxes[j])));
    let (Some(&first), Some(&last)) = (order.first(), order.last()) else { return out };
    let (first, last) = (&boxes[first], &boxes[last]);
    let span = pos(last) + size(last) - pos(first);
    let gap = (span - boxes.iter().map(size).sum::<f64>()) / (boxes.len() - 1) as f64;
    let mut cursor = pos(first);
    for i in order {
        let d = cursor - pos(&boxes[i]);
        out[i] = if horizontal { (d, 0.0) } else { (0.0, d) };
        cursor += size(&boxes[i]) + gap;
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snapped {
    Horizontal,
    Vertical,
    Diagonal,
}

/// Snaps `target` to horizontal or vertical around `origin` when drawing or
/// editing lines; with Shift, to 45° steps.
pub fn snap_to_right_angle(origin: (f64, f64), target: (f64, f64), enabled: bool, shift: bool) -> (f64, f64, Option<Snapped>) {
    let (threshold_px, threshold_deg) = (18.0, 7.0);
    let (dx, dy) = (target.0 - origin.0, target.1 - origin.1);
    let dist = dx.hypot(dy);
    if dist < 2.0 {
        return (target.0, target.1, None);
    }
    if shift {
        let step = std::f64::consts::PI / 4.0;
        let a = (dy.atan2(dx) / step).round() * step;
        let (nx, ny) = (origin.0 + (dist * a.cos()).round(), origin.1 + (dist * a.sin()).round());
        let s = if a.sin().abs() < 1e-4 {
            Snapped::Horizontal
        } else if a.cos().abs() < 1e-4 {
            Snapped::Vertical
        } else {
            Snapped::Diagonal
        };
        return (nx, ny, Some(s));
    }
    if !enabled {
        return (target.0, target.1, None);
    }
    let deg = (dy.atan2(dx) * 180.0 / std::f64::consts::PI).abs();
    let from_h = deg.min((180.0 - deg).abs());
    let from_v = (90.0 - from_h).abs();
    if from_h <= threshold_deg || (dy.abs() <= threshold_px && from_h <= 16.0) {
        return (target.0, origin.1, Some(Snapped::Horizontal));
    }
    if from_v <= threshold_deg || (dx.abs() <= threshold_px && from_v <= 16.0) {
        return (origin.0, target.1, Some(Snapped::Vertical));
    }
    (target.0, target.1, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> String {
        v.concat()
    }
    fn run(l: &str, pick: &[&str], m: LayerMove) -> String {
        let layers: Vec<String> = l.chars().map(String::from).collect();
        let pick: Vec<String> = pick.iter().map(|s| s.to_string()).collect();
        move_layers(&layers, |s: &String| s.as_str(), &pick, m).concat()
    }

    #[test]
    fn z_order() {
        assert_eq!(run("abc", &["a"], LayerMove::Front), "bca");
        assert_eq!(run("abc", &["c"], LayerMove::Back), "cab");
        assert_eq!(run("abc", &["a"], LayerMove::Forward), "bac");
        assert_eq!(run("abc", &["b"], LayerMove::Backward), "bac");
        assert_eq!(run("abc", &["c"], LayerMove::Forward), "abc");
        assert_eq!(run("abcde", &["b", "d"], LayerMove::Front), "acebd");
        assert_eq!(run("abcde", &["b", "d"], LayerMove::Back), "bdace");
        assert_eq!(run("abcde", &["b", "d"], LayerMove::Forward), "acbed");
        assert_eq!(run("abcde", &["b", "d"], LayerMove::Backward), "badce");
        assert_eq!(run("abcde", &["c", "d"], LayerMove::Forward), "abecd");
        assert_eq!(run("abcde", &["zz"], LayerMove::Front), "abcde");
        let abc: Vec<&str> = vec!["a", "b", "c"];
        assert_eq!(ids(&reorder_layer(&abc, 0, 2)), "bca");
        assert_eq!(ids(&reorder_layer(&abc, 2, 0)), "cab");
        assert_eq!(ids(&reorder_layer(&abc, 0, 99)), "bca");
    }

    #[test]
    fn snapping_and_aligning() {
        let s = snap(Box { x: 447.0, y: 212.0, width: 100.0, height: 50.0 }, &[], 1000.0, 1000.0, 8.0);
        assert_eq!(s, Snap { dx: 3.0, dy: 0.0, vertical: vec![500.0], horizontal: vec![] });
        let t = snap(Box { x: 305.0, y: 600.0, width: 50.0, height: 50.0 }, &[Box { x: 200.0, y: 0.0, width: 100.0, height: 20.0 }], 1000.0, 1000.0, 8.0);
        assert_eq!((t.dx, t.vertical), (-5.0, vec![300.0]));
        let b = Box { x: 100.0, y: 100.0, width: 200.0, height: 100.0 };
        assert_eq!(align_to_page(b, 1000.0, 800.0, PageAlign::Center), (300.0, 0.0));
        assert_eq!(align_to_page(b, 1000.0, 800.0, PageAlign::Bottom), (0.0, 600.0));
        assert_eq!(fit_scale(Box { x: 0.0, y: 0.0, width: 2000.0, height: 400.0 }, 1000.0, 800.0, 1.0), 0.5);
        assert_eq!(fit_scale(b, 1000.0, 800.0, 0.9), 1.0);
    }

    #[test]
    fn multi_selection_geometry() {
        let boxes = [
            Box { x: 0.0, y: 0.0, width: 10.0, height: 10.0 },
            Box { x: 50.0, y: 20.0, width: 20.0, height: 10.0 },
            Box { x: 100.0, y: 5.0, width: 10.0, height: 30.0 },
        ];
        assert_eq!(bounds_of(&boxes), Box { x: 0.0, y: 0.0, width: 110.0, height: 35.0 });
        assert!(intersects(boxes[0], Box { x: 5.0, y: 5.0, width: 1.0, height: 1.0 }));
        assert!(!intersects(boxes[0], Box { x: 10.0, y: 0.0, width: 5.0, height: 5.0 }));
        assert_eq!(align_boxes(&boxes, PageAlign::Left).iter().map(|d| d.0).collect::<Vec<_>>(), [0.0, -50.0, -100.0]);
        assert_eq!(align_boxes(&boxes, PageAlign::Right).iter().map(|d| d.0).collect::<Vec<_>>(), [100.0, 40.0, 0.0]);
        assert_eq!(distribute_boxes(&boxes, true).iter().map(|d| d.0).collect::<Vec<_>>(), [0.0, -5.0, 0.0]);
        let shuffled = [boxes[2], boxes[0], boxes[1]];
        assert_eq!(distribute_boxes(&shuffled, true).iter().map(|d| d.0).collect::<Vec<_>>(), [0.0, 0.0, -5.0]);
    }

    #[test]
    fn right_angles() {
        let o = (100.0, 100.0);
        assert_eq!(snap_to_right_angle(o, (300.0, 105.0), true, false), (300.0, 100.0, Some(Snapped::Horizontal)));
        assert_eq!(snap_to_right_angle(o, (-150.0, 96.0), true, false), (-150.0, 100.0, Some(Snapped::Horizontal)));
        assert_eq!(snap_to_right_angle(o, (106.0, 400.0), true, false), (100.0, 400.0, Some(Snapped::Vertical)));
        assert_eq!(snap_to_right_angle(o, (95.0, -100.0), true, false), (100.0, -100.0, Some(Snapped::Vertical)));
        assert_eq!(snap_to_right_angle(o, (250.0, 200.0), true, false), (250.0, 200.0, None));
        assert_eq!(snap_to_right_angle(o, (300.0, 105.0), false, false), (300.0, 105.0, None));
        let (x, y, s) = snap_to_right_angle(o, (200.0, 195.0), true, true);
        assert_eq!((s, x - o.0), (Some(Snapped::Diagonal), y - o.1));
    }
}
