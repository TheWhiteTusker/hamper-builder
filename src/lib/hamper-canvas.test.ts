import { test } from "node:test";
import assert from "node:assert/strict";
import {
  alignToPage,
  emptyCanvas,
  fillCss,
  fillProps,
  fitScale,
  layerLabel,
  moveLayer,
  record,
  redo,
  reorderLayer,
  snap,
  startHistory,
  undo,
} from "./hamper-canvas.ts";
import { parseCanvas } from "./hamper-canvas-schema.ts";

const ids = (layers: { id: string }[]) => layers.map((l) => l.id).join("");
const abc = [{ id: "a" }, { id: "b" }, { id: "c" }];

test("moveLayer reorders z-order and clamps at the ends", () => {
  assert.equal(ids(moveLayer(abc, "a", "front")), "bca");
  assert.equal(ids(moveLayer(abc, "c", "back")), "cab");
  assert.equal(ids(moveLayer(abc, "a", "forward")), "bac");
  assert.equal(ids(moveLayer(abc, "b", "backward")), "bac");
  assert.equal(moveLayer(abc, "c", "forward"), abc);
  assert.equal(moveLayer(abc, "a", "backward"), abc);
  assert.equal(moveLayer(abc, "zz", "front"), abc);
});

test("fillProps: solid is a plain fill, gradients span the box", () => {
  assert.deepEqual(fillProps({ type: "solid", color: "#ff0000" }, 10, 10), { fill: "#ff0000" });

  const h = fillProps({ type: "linear", from: "#000000", to: "#ffffff", angle: 0 }, 100, 50);
  assert.deepEqual(h.fillLinearGradientStartPoint, { x: 0, y: 25 });
  assert.deepEqual(h.fillLinearGradientEndPoint, { x: 100, y: 25 });

  const v = fillProps({ type: "linear", from: "#000000", to: "#ffffff", angle: 90 }, 40, 60, -20, -30);
  assert.ok(Math.abs(v.fillLinearGradientStartPoint!.x) < 1e-9);
  assert.ok(Math.abs(v.fillLinearGradientStartPoint!.y + 30) < 1e-9);
  assert.ok(Math.abs(v.fillLinearGradientEndPoint!.y - 30) < 1e-9);
});

test("parseCanvas round-trips a valid document and falls back on junk", () => {
  const doc = emptyCanvas();
  doc.layers.push(
    {
      id: "t1", visible: true, locked: false, letterSpacing: 0, lineHeight: 1, textDecoration: "none", kind: "text", x: 1, y: 2, rotation: 15, scaleX: 1, scaleY: 1, opacity: 1,
      text: "Hi", fontSize: 40, fontFamily: "Arial", fontStyle: "bold", align: "center", width: 300,
      fill: { type: "linear", from: "#111111", to: "#eeeeee", angle: 45 },
    },
    {
      id: "i1", visible: true, locked: false, fit: "stretch", kind: "image", x: 0, y: 0, rotation: 0, scaleX: 2, scaleY: 2, opacity: 0.5,
      product_id: null, url: "https://example.com/a.png", width: 200, height: 100,
    },
  );
  assert.deepEqual(parseCanvas(JSON.parse(JSON.stringify(doc))), doc);
  assert.deepEqual(parseCanvas(null), emptyCanvas());
  assert.deepEqual(parseCanvas({ layers: "nope" }), emptyCanvas());
});

test("history: undo/redo, grouping of continuous edits, redo cleared by new edit", () => {
  let h = startHistory(0);
  h = record(h, 1, null, 0);
  h = record(h, 2, "slider", 100);
  h = record(h, 3, "slider", 200);
  assert.deepEqual([h.past, h.present], [[0, 1], 3]);

  h = record(h, 4, "slider", 5000);
  assert.deepEqual(h.past, [0, 1, 3]);

  h = undo(undo(h));
  assert.equal(h.present, 1);
  assert.deepEqual(h.future, [3, 4]);
  h = redo(h);
  assert.equal(h.present, 3);

  h = record(h, 9, null, 6000);
  assert.deepEqual(h.future, []);
  assert.equal(record(h, 9, null, 7000), h);
  assert.equal(undo(startHistory(0)).present, 0);
});

test("snap: nearest edge/centre within threshold, per axis", () => {
  const canvas = { width: 1000, height: 1000 };
  const s = snap({ x: 447, y: 212, width: 100, height: 50 }, [], canvas, 8);
  assert.deepEqual(s, { dx: 3, dy: 0, vertical: [500], horizontal: [] });

  const t = snap({ x: 305, y: 600, width: 50, height: 50 }, [{ x: 200, y: 0, width: 100, height: 20 }], canvas, 8);
  assert.equal(t.dx, -5);
  assert.deepEqual(t.vertical, [300]);
});

test("reorderLayer puts the layer at the target index from either direction", () => {
  assert.equal(ids(reorderLayer(abc, "a", 2)), "bca");
  assert.equal(ids(reorderLayer(abc, "c", 0)), "cab");
  assert.equal(ids(reorderLayer(abc, "a", 1)), "bac");
  assert.equal(ids(reorderLayer(abc, "a", 99)), "bca");
  assert.equal(reorderLayer(abc, "b", 1), abc);
});

test("alignToPage and fitScale", () => {
  const page = { width: 1000, height: 800 };
  const box = { x: 100, y: 100, width: 200, height: 100 };
  assert.deepEqual(alignToPage(box, page, "left"), { dx: -100, dy: 0 });
  assert.deepEqual(alignToPage(box, page, "center"), { dx: 300, dy: 0 });
  assert.deepEqual(alignToPage(box, page, "right"), { dx: 700, dy: 0 });
  assert.deepEqual(alignToPage(box, page, "middle"), { dx: 0, dy: 250 });
  assert.deepEqual(alignToPage(box, page, "bottom"), { dx: 0, dy: 600 });
  assert.equal(fitScale({ x: 0, y: 0, width: 2000, height: 400 }, page, 1), 0.5);
  assert.equal(fitScale(box, page), 1);
});

test("old saved designs get the new layer defaults", () => {
  const old = {
    ...emptyCanvas(),
    layers: [
      {
        id: "t", kind: "text", x: 0, y: 0, rotation: 0, scaleX: 1, scaleY: 1, opacity: 1,
        text: "Hi", fontSize: 10, fontFamily: "Arial", fontStyle: "normal", align: "left", width: 50,
        fill: { type: "solid", color: "#000000" },
      },
    ],
  };
  const [t] = parseCanvas(old).layers;
  assert.equal(t.visible, true);
  assert.equal(t.locked, false);
  assert.equal(t.kind === "text" && t.lineHeight, 1);
  assert.equal(layerLabel(t), "Hi");
  assert.equal(fillCss({ type: "linear", from: "#000000", to: "#ffffff", angle: 0 }), "linear-gradient(90deg, #000000, #ffffff)");
});
