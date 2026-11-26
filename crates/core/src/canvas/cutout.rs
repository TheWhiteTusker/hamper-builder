//! Makes a product photo's plain background transparent, in place (port of
//! `background-removal.ts`).
//!
//! Flood-fills inward from the border through every pixel close to the
//! border colour, so a white product on a white backdrop keeps its inside.
//! Pixels just past the tolerance that touch the removed area are feathered.
//!
//! ponytail: colour-distance flood fill, so only plain or near-plain
//! backdrops; busy photographic backgrounds need a matting model.

/// `rgba` is width×height×4 bytes, as `getImageData` gives it.
// Pixel indices stay within `0..width * height`, checked against the buffer above.
#[allow(clippy::indexing_slicing)]
pub fn remove_background(rgba: &mut [u8], width: usize, height: usize, tolerance: f64) {
    let n = width * height;
    if n == 0 || rgba.len() < n * 4 {
        return;
    }
    let px = |rgba: &[u8], i: usize, c: usize| f64::from(rgba.get(i * 4 + c).copied().unwrap_or(0));
    // Average of the four corners: the backdrop colour on a typical product shot.
    let corners = [0, width - 1, (height - 1) * width, n - 1];
    let key: Vec<f64> = (0..3).map(|c| corners.iter().map(|&i| px(rgba, i, c)).sum::<f64>() / 4.0).collect();
    let dist = |rgba: &[u8], i: usize| {
        if px(rgba, i, 3) == 0.0 {
            return 0.0; // already transparent counts as background
        }
        let d: f64 = (0..3).map(|c| (px(rgba, i, c) - key.get(c).copied().unwrap_or(0.0)).powi(2)).sum();
        d.sqrt()
    };
    let feather = tolerance * 0.5;
    let mut removed = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let push = |i: usize, removed: &mut Vec<bool>, stack: &mut Vec<usize>| {
        if removed.get(i) == Some(&false) && dist(rgba, i) <= tolerance {
            removed[i] = true;
            stack.push(i);
        }
    };
    for x in 0..width {
        push(x, &mut removed, &mut stack);
        push((height - 1) * width + x, &mut removed, &mut stack);
    }
    for y in 0..height {
        push(y * width, &mut removed, &mut stack);
        push(y * width + width - 1, &mut removed, &mut stack);
    }
    while let Some(i) = stack.pop() {
        let x = i % width;
        if x > 0 {
            push(i - 1, &mut removed, &mut stack);
        }
        if x < width - 1 {
            push(i + 1, &mut removed, &mut stack);
        }
        if i >= width {
            push(i - width, &mut removed, &mut stack);
        }
        if i < n - width {
            push(i + width, &mut removed, &mut stack);
        }
    }
    let alphas: Vec<Option<u8>> = (0..n)
        .map(|i| {
            if removed[i] {
                return Some(0);
            }
            if feather <= 0.0 {
                return None;
            }
            let x = i % width;
            let touches = (x > 0 && removed[i - 1])
                || (x < width - 1 && removed[i + 1])
                || (i >= width && removed[i - width])
                || (i < n - width && removed[i + width]);
            let d = dist(rgba, i) - tolerance;
            (touches && d < feather).then(|| crate::js::round(px(rgba, i, 3) * d / feather) as u8)
        })
        .collect();
    for (i, a) in alphas.into_iter().enumerate() {
        if let (Some(a), Some(slot)) = (a, rgba.get_mut(i * 4 + 3)) {
            *slot = a;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_backdrop_goes_but_a_white_inside_stays() {
        // 5×5 white, a black ring at the middle 3×3, white in its centre.
        let mut px = vec![255u8; 5 * 5 * 4];
        for y in 1..4 {
            for x in 1..4 {
                if (x, y) != (2, 2) {
                    let i = (y * 5 + x) * 4;
                    px[i..i + 3].copy_from_slice(&[0, 0, 0]);
                }
            }
        }
        remove_background(&mut px, 5, 5, 30.0);
        assert_eq!(px[3], 0, "corner is transparent");
        assert_eq!(px[(2 * 5 + 2) * 4 + 3], 255, "the enclosed white centre is kept");
        assert_eq!(px[(5 + 1) * 4 + 3], 255, "the ring is kept");
    }
}
