import type { Layer } from "../hamper-canvas.ts";

type ImageLayer = Extract<Layer, { kind: "image" }>;

/**
 * Maps source-pixel positions in a split image onto the page, so the pieces
 * land exactly where they were drawn inside the original image layer,
 * whatever its fit, scale, flip or rotation.
 *
 * Children share the parent's rotation and scale; sizes come back in the
 * parent's frame units (multiply source pixels by kx / ky).
 */
export function framePlacer(parent: ImageLayer, natW: number, natH: number) {
  let kx = parent.width / natW;
  let ky = parent.height / natH;
  let ox = 0;
  let oy = 0;
  if (parent.fit === "contain") {
    kx = ky = Math.min(kx, ky);
    ox = (parent.width - natW * kx) / 2;
    oy = (parent.height - natH * ky) / 2;
  }
  const rad = (parent.rotation * Math.PI) / 180;
  const cos = Math.cos(rad);
  const sin = Math.sin(rad);
  /** Page position of source pixel (px, py). */
  const at = (px: number, py: number) => {
    const lx = (ox + px * kx) * parent.scaleX;
    const ly = (oy + py * ky) * parent.scaleY;
    return { x: parent.x + lx * cos - ly * sin, y: parent.y + lx * sin + ly * cos };
  };
  const transform = {
    rotation: parent.rotation,
    scaleX: parent.scaleX,
    scaleY: parent.scaleY,
    opacity: parent.opacity,
  };
  return { at, kx, ky, transform };
}

/**
 * A font size that makes `text` as wide as the box OCR found it in, starting
 * from the row height and staying within ±30% of it (a different typeface
 * can't be matched exactly, and a wild size is worse than a near miss).
 */
export function fitFontSize(rowHeight: number, boxWidth: number, widthAt100: number) {
  if (widthAt100 <= 0) return rowHeight;
  const fitted = (100 * boxWidth) / widthAt100;
  return Math.min(rowHeight * 1.3, Math.max(rowHeight * 0.7, fitted));
}
