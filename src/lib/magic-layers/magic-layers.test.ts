import { test } from "node:test";
import assert from "node:assert/strict";
import { alphaFor, pickObjects, shareInside, sourceBox, type LowMask } from "./masks.ts";
import { dilate, fillHoles } from "./fill.ts";
import { textStyle } from "./text-style.ts";
import { fitFontSize, framePlacer } from "./to-layers.ts";
import type { Layer } from "../hamper-canvas.ts";

/** A low-res mask from rows of chars: # inside (logit +8), . outside (−8). */
function mask(rows: string[], score = 0.95): LowMask {
  const logits = Float32Array.from(rows.join("").split(""), (c) => (c === "#" ? 8 : -8));
  return { logits, w: rows[0].length, h: rows.length, score };
}

/** An RGBA image from rows of chars: W white, K black, R red. */
function image(rows: string[]) {
  const colors: Record<string, number[]> = { W: [255, 255, 255], K: [0, 0, 0], R: [200, 0, 0] };
  const data = new Uint8ClampedArray(rows.length * rows[0].length * 4);
  rows.join("").split("").forEach((ch, i) => data.set([...colors[ch], 255], i * 4));
  return { data, w: rows[0].length, h: rows.length };
}

const big = mask(["......", ".####.", ".####.", ".####.", ".####.", "......"]);
const part = mask(["......", ".##...", ".##...", "......", "......", "......"], 0.99);
const twin = mask(["......", ".####.", ".####.", ".####.", ".###..", "......"], 0.9);
const backdrop = mask(["######", "######", "######", "######", "######", "#####."]);
const speck = mask(["......", "......", "..#...", "......", "......", "......"]);
const shy = mask(["####..", "####..", "......", "......", "......", "......"], 0.5);

test("pickObjects drops backdrops, specks, low scores, duplicates and parts of kept objects", () => {
  const picked = pickObjects([part, twin, big, backdrop, speck, shy], { minArea: 0.05, maxArea: 0.6 });
  assert.deepEqual(picked, [big]);
});

test("pickObjects keeps separate objects, best score first", () => {
  const left = mask(["##....", "##....", "......", "......", "......", "......"], 0.9);
  const right = mask(["......", "......", "......", "......", "....##", "....##"], 0.97);
  assert.deepEqual(pickObjects([left, right], { minArea: 0.05 }), [right, left]);
});

test("shareInside measures how much of a mask OCR already covers", () => {
  assert.equal(shareInside(big, [{ x: 1, y: 1, w: 4, h: 2 }]), 0.5);
  assert.equal(shareInside(big, []), 0);
});

test("sourceBox scales the mask bounds to source pixels with padding, clamped to the image", () => {
  // 6 low-res cells over a 60px image: 0.1 cells per pixel.
  assert.deepEqual(sourceBox(big, 0.1, 60, 60, 2), { x: 8, y: 8, w: 44, h: 44 });
  assert.deepEqual(sourceBox(backdrop, 0.1, 60, 60, 5), { x: 0, y: 0, w: 60, h: 60 });
  assert.equal(sourceBox(mask(["..", ".."]), 1, 2, 2), null);
});

test("alphaFor is solid inside, clear outside and soft across the edge", () => {
  const a = alphaFor(big, 0.1, { x: 0, y: 30, w: 60, h: 1 });
  assert.equal(a[30], 255);
  assert.equal(a[0], 0);
  const edge = Array.from(a.slice(5, 16));
  assert.ok(edge.some((v) => v > 0 && v < 255), "edge has partial alpha");
  assert.deepEqual(edge, [...edge].sort((x, y) => x - y), "edge ramps up");
});

test("fillHoles fills a hole in a flat colour exactly", () => {
  const { data, w, h } = image(["RRRRR", "RWWWR", "RWWWR", "RRRRR"]);
  const hole = Uint8Array.from("00000011100111000000", Number);
  fillHoles(data, w, h, hole);
  for (let i = 0; i < w * h; i++) assert.deepEqual(Array.from(data.slice(i * 4, i * 4 + 4)), [200, 0, 0, 255]);
});

test("fillHoles blends between differing sides", () => {
  const { data, w, h } = image(["KWR"]);
  fillHoles(data, w, h, Uint8Array.from([0, 1, 0]));
  assert.deepEqual(Array.from(data.slice(4, 8)), [100, 0, 0, 255]);
});

test("dilate grows a mask by r in every direction", () => {
  const m = new Uint8Array(25);
  m[12] = 1;
  assert.deepEqual(Array.from(dilate(m, 5, 5, 1)).join(""), "0000001110011100111000000");
});

test("textStyle reads the ink colour and weight off the backdrop", () => {
  const thin = image(["WWWWWWWW", "WKWWWWKW", "WKWWWWKW", "WWWWWWWW"]);
  assert.deepEqual(textStyle(thin.data, thin.w, { x0: 0, y0: 0, x1: 8, y1: 4 }), { color: "#000000", bold: false });
  const heavy = image(["WWWWWWWW", "WRRRRRRW", "WRRRRRRW", "WWWWWWWW"]);
  assert.deepEqual(textStyle(heavy.data, heavy.w, { x0: 0, y0: 0, x1: 8, y1: 4 }), { color: "#c80000", bold: true });
});

const parent = (p: Partial<Extract<Layer, { kind: "image" }>>): Extract<Layer, { kind: "image" }> => ({
  id: "p", visible: true, locked: false, x: 100, y: 50, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1,
  kind: "image", product_id: null, url: "https://x.test/a.png", width: 200, height: 100, fit: "stretch", ...p,
});

const close = (a: { x: number; y: number }, b: { x: number; y: number }) =>
  assert.ok(Math.abs(a.x - b.x) < 1e-9 && Math.abs(a.y - b.y) < 1e-9, `${JSON.stringify(a)} ≈ ${JSON.stringify(b)}`);

test("framePlacer maps source pixels through a stretched frame", () => {
  const p = framePlacer(parent({}), 400, 400);
  assert.equal(p.kx, 0.5);
  assert.equal(p.ky, 0.25);
  close(p.at(200, 200), { x: 200, y: 100 });
});

test("framePlacer centres a contain-fit image and follows rotation and scale", () => {
  const contain = framePlacer(parent({ fit: "contain" }), 400, 400);
  assert.equal(contain.kx, 0.25);
  close(contain.at(0, 0), { x: 150, y: 50 });

  const turned = framePlacer(parent({ rotation: 90, scaleX: 2 }), 200, 100);
  close(turned.at(10, 0), { x: 100, y: 70 });
  close(turned.at(0, 10), { x: 90, y: 50 });
  assert.deepEqual(turned.transform, { rotation: 90, scaleX: 2, scaleY: 1, opacity: 1 });
});

test("fitFontSize matches the box width within ±30% of the row height", () => {
  assert.equal(fitFontSize(40, 200, 500), 40);
  assert.equal(fitFontSize(40, 220, 500), 44);
  assert.equal(fitFontSize(40, 1000, 500), 52);
  assert.equal(fitFontSize(40, 10, 500), 28);
});
