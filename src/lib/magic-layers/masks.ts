/**
 * Picking object masks for Magic Layers, and turning them into soft alpha.
 *
 * SAM's decoder returns low-resolution logits (> 0 is inside the object). We
 * choose and dedupe masks at that resolution, then upscale only the survivors
 * against the full-resolution photo, so cut-outs keep every original pixel.
 */

/** One decoder mask: w×h logits and the model's own quality estimate. */
export type LowMask = { logits: Float32Array; w: number; h: number; score: number };

/** A rectangle in source pixels. */
export type Box = { x: number; y: number; w: number; h: number };

export const bits = (m: LowMask) => m.logits.map((v) => (v > 0 ? 1 : 0));

const count = (b: ArrayLike<number>) => {
  let n = 0;
  for (let i = 0; i < b.length; i++) n += b[i];
  return n;
};

const overlap = (a: ArrayLike<number>, b: ArrayLike<number>) => {
  let n = 0;
  for (let i = 0; i < a.length; i++) n += a[i] & b[i];
  return n;
};

/**
 * The masks worth turning into layers: confident, neither specks nor the
 * whole backdrop, no near-duplicates, and no pieces of an object already kept
 * (a bottle's label goes with the bottle). Best first.
 */
export function pickObjects(
  masks: LowMask[],
  { minScore = 0.88, minArea = 0.003, maxArea = 0.4, maxIou = 0.7, contained = 0.85 } = {},
): LowMask[] {
  const scored = masks
    .map((m) => ({ m, b: bits(m) }))
    .map((x) => ({ ...x, area: count(x.b) }))
    .filter(({ m, area }) => m.score >= minScore && area >= minArea * m.w * m.h && area <= maxArea * m.w * m.h)
    .sort((a, b) => b.m.score - a.m.score);

  const kept: typeof scored = [];
  for (const c of scored) {
    const dupe = kept.some((k) => {
      const i = overlap(c.b, k.b);
      return i / (c.area + k.area - i) > maxIou;
    });
    if (!dupe) kept.push(c);
  }
  return kept
    .filter((c) => !kept.some((k) => k !== c && k.area > c.area && overlap(c.b, k.b) / c.area >= contained))
    .map((c) => c.m);
}

/** Share of the mask that falls inside any of `boxes` (low-res cells). Text found by OCR shouldn't also become an object. */
export function shareInside(m: LowMask, boxes: Box[]): number {
  const b = bits(m);
  let total = 0;
  let inside = 0;
  for (let y = 0; y < m.h; y++) {
    for (let x = 0; x < m.w; x++) {
      if (!b[y * m.w + x]) continue;
      total++;
      if (boxes.some((r) => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)) inside++;
    }
  }
  return total ? inside / total : 0;
}

/**
 * Bounding box of the mask in source pixels, padded by `pad`. `scale` is
 * low-res cells per source pixel.
 */
export function sourceBox(m: LowMask, scale: number, srcW: number, srcH: number, pad = 2): Box | null {
  const b = bits(m);
  let x0 = m.w, y0 = m.h, x1 = -1, y1 = -1;
  for (let y = 0; y < m.h; y++) {
    for (let x = 0; x < m.w; x++) {
      if (!b[y * m.w + x]) continue;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
    }
  }
  if (x1 < 0) return null;
  const left = Math.max(0, Math.floor(x0 / scale) - pad);
  const top = Math.max(0, Math.floor(y0 / scale) - pad);
  const right = Math.min(srcW, Math.ceil((x1 + 1) / scale) + pad);
  const bottom = Math.min(srcH, Math.ceil((y1 + 1) / scale) + pad);
  return { x: left, y: top, w: right - left, h: bottom - top };
}

/**
 * Alpha (0–255) for every source pixel in `box`: the logits sampled
 * bilinearly at full resolution and passed through a sigmoid, so the edge is
 * soft rather than stair-stepped.
 */
export function alphaFor(m: LowMask, scale: number, box: Box): Uint8ClampedArray {
  const out = new Uint8ClampedArray(box.w * box.h);
  const at = (x: number, y: number) =>
    m.logits[Math.min(m.h - 1, Math.max(0, y)) * m.w + Math.min(m.w - 1, Math.max(0, x))];
  for (let j = 0; j < box.h; j++) {
    const v = (box.y + j + 0.5) * scale - 0.5;
    const y0 = Math.floor(v);
    const fy = v - y0;
    for (let i = 0; i < box.w; i++) {
      const u = (box.x + i + 0.5) * scale - 0.5;
      const x0 = Math.floor(u);
      const fx = u - x0;
      const top = at(x0, y0) * (1 - fx) + at(x0 + 1, y0) * fx;
      const bottom = at(x0, y0 + 1) * (1 - fx) + at(x0 + 1, y0 + 1) * fx;
      out[j * box.w + i] = 255 / (1 + Math.exp(-(top * (1 - fy) + bottom * fy)));
    }
  }
  return out;
}
