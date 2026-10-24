/*
 * The hamper image designer's document. Stored as-is in hampers.canvas.
 * Layer array order is z-order: the last layer is drawn on top.
 */

export const CANVAS_PRESETS = [
  { label: "Square 1:1", width: 1080, height: 1080 },
  { label: "Portrait 4:5", width: 1080, height: 1350 },
  { label: "Story 9:16", width: 1080, height: 1920 },
  { label: "Landscape 16:9", width: 1920, height: 1080 },
  { label: "A4 portrait", width: 1240, height: 1754 },
] as const;

import type { Fill, HamperCanvas, Layer } from "./hamper-canvas-schema.ts";
export type { Fill, Layer, LayerKind, HamperCanvas } from "./hamper-canvas-schema.ts";

export const emptyCanvas = (): HamperCanvas => ({
  width: CANVAS_PRESETS[0].width,
  height: CANVAS_PRESETS[0].height,
  background: { fill: { type: "solid", color: "#ffffff" }, image_url: null },
  layers: [],
});

/** What the Layers panel calls a layer when it has not been renamed. */
export function layerLabel(l: Layer): string {
  if (l.name) return l.name;
  switch (l.kind) {
    case "image":
      return "Image";
    case "text":
      return l.text.trim().split("\n")[0].slice(0, 40) || "Text";
    case "rect":
      return l.cornerRadius > 0 ? "Rounded rectangle" : "Rectangle";
    case "ellipse":
      return "Circle";
    case "polygon":
      return l.sides === 3 ? "Triangle" : l.sides === 6 ? "Hexagon" : `${l.sides}-sided shape`;
    case "star":
      return "Star";
    case "line":
      return "Line";
    case "curve":
      return "Curved line";
  }
}

/** The fill as CSS, for swatches and layer thumbnails. */
export const fillCss = (fill: Fill) =>
  fill.type === "solid" ? fill.color : `linear-gradient(${fill.angle + 90}deg, ${fill.from}, ${fill.to})`;

export * from "./canvas-render.ts";
export * from "./canvas-history.ts";
export * from "./canvas-arrange.ts";
export * from "./canvas-snap-angle.ts";
