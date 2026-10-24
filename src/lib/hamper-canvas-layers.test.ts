import { test } from "node:test";
import assert from "node:assert/strict";
import {
  alignBoxes,
  backgroundImageAttrs,
  boundsOf,
  distributeBoxes,
  intersects,
  layerConfig,
  moveLayers,
  snapToRightAngle,
} from "./hamper-canvas.ts";
import { parseCanvas } from "./hamper-canvas-schema.ts";

const ids = (layers: { id: string }[]) => layers.map((l) => l.id).join("");

test("moveLayers keeps the selection's own order", () => {
  const l = ["a", "b", "c", "d", "e"].map((id) => ({ id }));
  assert.equal(ids(moveLayers(l, ["b", "d"], "front")), "acebd");
  assert.equal(ids(moveLayers(l, ["b", "d"], "back")), "bdace");
  assert.equal(ids(moveLayers(l, ["b", "d"], "forward")), "acbed");
  assert.equal(ids(moveLayers(l, ["b", "d"], "backward")), "badce");
  assert.equal(ids(moveLayers(l, ["c", "d"], "forward")), "abecd");
  assert.equal(moveLayers(l, ["d", "e"], "forward"), l);
  assert.equal(moveLayers(l, ["d", "e"], "front"), l);
  assert.equal(moveLayers(l, ["zz"], "front"), l);
});

test("multi-selection geometry: bounds, hit test, align and distribute", () => {
  const boxes = [
    { x: 0, y: 0, width: 10, height: 10 },
    { x: 50, y: 20, width: 20, height: 10 },
    { x: 100, y: 5, width: 10, height: 30 },
  ];
  assert.deepEqual(boundsOf(boxes), { x: 0, y: 0, width: 110, height: 35 });
  assert.equal(intersects(boxes[0], { x: 5, y: 5, width: 1, height: 1 }), true);
  assert.equal(intersects(boxes[0], { x: 10, y: 0, width: 5, height: 5 }), false);

  assert.deepEqual(alignBoxes(boxes, "left").map((d) => d.dx), [0, -50, -100]);
  assert.deepEqual(alignBoxes(boxes, "right").map((d) => d.dx), [100, 40, 0]);
  assert.deepEqual(alignBoxes(boxes, "top").map((d) => d.dy), [0, -20, -5]);

  assert.deepEqual(distributeBoxes(boxes, "x").map((d) => d.dx), [0, -5, 0]);
  const shuffled = [boxes[2], boxes[0], boxes[1]];
  assert.deepEqual(distributeBoxes(shuffled, "x").map((d) => d.dx), [0, 0, -5]);
  assert.deepEqual(distributeBoxes(boxes.slice(0, 2), "x"), [{ dx: 0, dy: 0 }, { dx: 0, dy: 0 }]);
});

test("layerConfig: contain fits and centres the photo inside its frame", () => {
  const base = { id: "i", visible: true, locked: false, x: 10, y: 20, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1, product_id: null, url: "https://e.com/a.png" };
  const tall = { naturalWidth: 100, naturalHeight: 200 };
  const c = layerConfig({ ...base, kind: "image", width: 400, height: 400, fit: "contain" }, tall);
  assert.equal(c.shape, "Image");
  assert.deepEqual([c.attrs.width, c.attrs.height, c.attrs.offsetX, c.attrs.offsetY], [200, 400, -100, -0]);
  const s = layerConfig({ ...base, kind: "image", width: 400, height: 400, fit: "stretch" }, tall);
  assert.deepEqual([s.attrs.width, s.attrs.height, s.attrs.offsetX], [400, 400, undefined]);

  assert.deepEqual(backgroundImageAttrs({ width: 1000, height: 500 }, { naturalWidth: 100, naturalHeight: 100 }), {
    width: 1000, height: 1000, x: 0, y: -250,
  });
});

test("layerConfig and parseCanvas: line and curve layers", () => {
  const base = { id: "l1", visible: true, locked: false, x: 50, y: 60, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1 };
  const lineDoc = {
    width: 1080,
    height: 1080,
    background: { fill: { type: "solid", color: "#ffffff" }, image_url: null },
    layers: [
      { ...base, kind: "line", points: [0, 0, 200, 100], stroke: "#2c332f", strokeWidth: 4, lineCap: "round" },
      { ...base, id: "c1", kind: "curve", points: [0, 0, 100, -50, 200, 0], curvature: 0.5, stroke: "#54655b", strokeWidth: 6, lineCap: "round" },
    ],
  };

  const parsed = parseCanvas(lineDoc);
  assert.equal(parsed.layers.length, 2);
  assert.equal(parsed.layers[0].kind, "line");
  assert.equal(parsed.layers[1].kind, "curve");

  const lineCfg = layerConfig(parsed.layers[0]);
  assert.equal(lineCfg.shape, "Line");
  assert.equal(lineCfg.attrs.stroke, "#2c332f");
  assert.equal(lineCfg.attrs.strokeWidth, 4);
  assert.equal(lineCfg.attrs.hitStrokeWidth, 28);
  assert.deepEqual(lineCfg.attrs.points, [0, 0, 200, 100]);

  const curveCfg = layerConfig(parsed.layers[1]);
  assert.equal(curveCfg.shape, "Line");
  assert.equal(curveCfg.attrs.stroke, "#54655b");
  assert.equal(curveCfg.attrs.strokeWidth, 6);
  assert.equal(curveCfg.attrs.hitStrokeWidth, 28);
  assert.equal(curveCfg.attrs.bezier, true);
  assert.deepEqual(curveCfg.attrs.points, [0, 0, 100, -50, 200, 0]);
});

test("snapToRightAngle snaps close to horizontal and vertical lines", () => {
  const origin = { x: 100, y: 100 };

  const nearHoriz = snapToRightAngle(origin, { x: 300, y: 105 });
  assert.equal(nearHoriz.snapped, "horizontal");
  assert.equal(nearHoriz.y, 100);
  assert.equal(nearHoriz.x, 300);

  const nearHorizLeft = snapToRightAngle(origin, { x: -150, y: 96 });
  assert.equal(nearHorizLeft.snapped, "horizontal");
  assert.equal(nearHorizLeft.y, 100);
  assert.equal(nearHorizLeft.x, -150);

  const nearVert = snapToRightAngle(origin, { x: 106, y: 400 });
  assert.equal(nearVert.snapped, "vertical");
  assert.equal(nearVert.x, 100);
  assert.equal(nearVert.y, 400);

  const nearVertUp = snapToRightAngle(origin, { x: 95, y: -100 });
  assert.equal(nearVertUp.snapped, "vertical");
  assert.equal(nearVertUp.x, 100);
  assert.equal(nearVertUp.y, -100);

  const freeform = snapToRightAngle(origin, { x: 250, y: 200 });
  assert.equal(freeform.snapped, null);
  assert.equal(freeform.x, 250);
  assert.equal(freeform.y, 200);

  const disabled = snapToRightAngle(origin, { x: 300, y: 105 }, { enabled: false });
  assert.equal(disabled.snapped, null);
  assert.equal(disabled.y, 105);

  const shiftDiagonal = snapToRightAngle(origin, { x: 200, y: 195 }, { shiftKey: true });
  assert.equal(shiftDiagonal.snapped, "diagonal");
  assert.equal(shiftDiagonal.x - origin.x, shiftDiagonal.y - origin.y);
});
