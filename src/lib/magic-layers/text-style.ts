/** A text line's box in source pixels, as OCR reports it (x1, y1 exclusive). */
export type LineBox = { x0: number; y0: number; x1: number; y1: number };

const hex = (rgb: number[]) => `#${rgb.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;

/**
 * The colour and weight of the text inside `box`.
 *
 * The box's edge is mostly backdrop, so pixels far from the edge colour are
 * ink. The text colour is the average of the clearest ink (anti-aliased
 * pixels blend toward the backdrop, so they're left out).
 *
 * ponytail: "bold" is ink coverage over 30% of a tight line box. Good for
 * clear headings; the user fixes the odd miss with the B button.
 */
export function textStyle(data: Uint8ClampedArray, w: number, box: LineBox): { color: string; bold: boolean } {
  const { x0, y0, x1, y1 } = box;
  const px = (x: number, y: number) => {
    const p = (y * w + x) * 4;
    return [data[p], data[p + 1], data[p + 2]];
  };

  const edge = [0, 0, 0];
  let edges = 0;
  for (let x = x0; x < x1; x++) {
    for (const y of [y0, y1 - 1]) {
      px(x, y).forEach((v, c) => (edge[c] += v));
      edges++;
    }
  }
  for (let y = y0 + 1; y < y1 - 1; y++) {
    for (const x of [x0, x1 - 1]) {
      px(x, y).forEach((v, c) => (edge[c] += v));
      edges++;
    }
  }
  const bg = edge.map((v) => v / Math.max(1, edges));

  const dist: number[] = [];
  let max = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const [r, g, b] = px(x, y);
      const d = Math.hypot(r - bg[0], g - bg[1], b - bg[2]);
      dist.push(d);
      if (d > max) max = d;
    }
  }
  if (max < 48) return { color: "#000000", bold: false };

  const ink = [0, 0, 0];
  let clear = 0;
  let inked = 0;
  let i = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++, i++) {
      if (dist[i] >= 48) inked++;
      if (dist[i] < max * 0.6) continue;
      px(x, y).forEach((v, c) => (ink[c] += v));
      clear++;
    }
  }
  return { color: hex(ink.map((v) => v / clear)), bold: inked / dist.length > 0.3 };
}
