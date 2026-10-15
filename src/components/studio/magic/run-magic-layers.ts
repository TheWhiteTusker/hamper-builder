import type { Layer } from "@/lib/hamper-canvas";
import { dilate, fillHoles } from "@/lib/magic-layers/fill";
import { alphaFor, pickObjects, shareInside, sourceBox, type Box } from "@/lib/magic-layers/masks";
import { textStyle, type LineBox } from "@/lib/magic-layers/text-style";
import { fitFontSize, framePlacer } from "@/lib/magic-layers/to-layers";
import { baseLayer, type UploadTicket } from "../editor";
import { uploadDirect } from "../image-utils";
import { ensureFontStylesheet, loadImage } from "../render";
import { readLines } from "./ocr";
import { findObjects } from "./sam";

type ImageLayer = Extract<Layer, { kind: "image" }>;
export type MagicStep = { label: string; progress?: number };

/** Detected text is set in Inter; the user restyles it like any other text. */
const FONT = "Inter";

function blankCanvas(w: number, h: number) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return { c, ctx: c.getContext("2d", { willReadFrequently: true })! };
}

const markBox = (mask: Uint8Array, w: number, b: LineBox, pad: number) => {
  for (let y = Math.max(0, b.y0 - pad); y < Math.min(mask.length / w, b.y1 + pad); y++) {
    for (let x = Math.max(0, b.x0 - pad); x < Math.min(w, b.x1 + pad); x++) mask[y * w + x] = 1;
  }
};

/**
 * Splits an image layer into a filled-in background, a cut-out per object and
 * an editable text layer per line, placed exactly over the original. Pixels
 * come from the full-size original; the models only guide where to cut.
 * Returns [] when there's nothing to separate.
 */
export async function runMagicLayers(
  parent: ImageLayer,
  onUpload: (ext: string) => Promise<UploadTicket>,
  onStep: (s: MagicStep) => void,
): Promise<Layer[]> {
  onStep({ label: "Loading image" });
  const img = await loadImage(parent.url);
  const W = img.naturalWidth;
  const H = img.naturalHeight;
  const src = blankCanvas(W, H);
  src.ctx.drawImage(img, 0, 0);
  const original = src.ctx.getImageData(0, 0, W, H);

  onStep({ label: "Reading text (first time downloads the OCR engine)", progress: 0 });
  const lines = (await readLines(src.c, (p) => onStep({ label: "Reading text", progress: p }))).map((l) => ({
    ...l,
    box: { x0: l.box.x0, y0: l.box.y0, x1: Math.min(W, l.box.x1), y1: Math.min(H, l.box.y1) },
  }));

  onStep({ label: "Finding objects (first time downloads a 14 MB model)", progress: 0 });
  const { masks, scale } = await findObjects(src.c, (p) => onStep({ label: "Finding objects", progress: p }));
  const textCells: Box[] = lines.map(({ box: b }) => ({
    x: b.x0 * scale, y: b.y0 * scale, w: (b.x1 - b.x0) * scale, h: (b.y1 - b.y0) * scale,
  }));
  const objects = pickObjects(masks).filter((m) => shareInside(m, textCells) < 0.5);
  if (!lines.length && !objects.length) return [];

  onStep({ label: "Separating layers" });
  // Read text colours before anything is painted over.
  const styles = lines.map((l) => textStyle(original.data, W, l.box));
  const textHole = new Uint8Array(W * H);
  lines.forEach((l) => markBox(textHole, W, l.box, 3));
  const hole = textHole.slice();

  const cutouts = objects.flatMap((m) => {
    const box = sourceBox(m, scale, W, H);
    if (!box) return [];
    const alpha = alphaFor(m, scale, box);
    const piece = blankCanvas(box.w, box.h);
    const px = src.ctx.getImageData(box.x, box.y, box.w, box.h);
    // Text printed on the object becomes its own layer, so lift it off the cut-out too.
    const onPiece = new Uint8Array(box.w * box.h);
    for (let y = 0; y < box.h; y++) for (let x = 0; x < box.w; x++) onPiece[y * box.w + x] = textHole[(box.y + y) * W + box.x + x];
    if (onPiece.includes(1)) fillHoles(px.data, box.w, box.h, onPiece);
    for (let i = 0; i < alpha.length; i++) {
      px.data[i * 4 + 3] = Math.min(px.data[i * 4 + 3], alpha[i]);
      if (alpha[i] > 64) hole[(box.y + Math.floor(i / box.w)) * W + box.x + (i % box.w)] = 1;
    }
    piece.ctx.putImageData(px, 0, 0);
    return [{ box, canvas: piece.c }];
  });
  // Biggest first, so smaller objects sit on top of the ones they overlap.
  cutouts.sort((a, b) => b.box.w * b.box.h - a.box.w * a.box.h);

  fillHoles(original.data, W, H, dilate(hole, W, H, 3));
  src.ctx.putImageData(original, 0, 0);

  onStep({ label: "Saving layers" });
  const upload = async (c: HTMLCanvasElement, name: string) => {
    const blob = await new Promise<Blob | null>((r) => c.toBlob(r, "image/png"));
    if (!blob) throw new Error("Could not render a layer.");
    const res = await uploadDirect(onUpload, new File([blob], `${name}.png`, { type: "image/png" }));
    if (res.error || !res.url) throw new Error(res.error ?? "Upload failed.");
    return res.url;
  };
  const [bgUrl, ...pieceUrls] = await Promise.all([
    upload(src.c, "background"),
    ...cutouts.map((p, i) => upload(p.canvas, `object-${i + 1}`)),
  ]);

  const place = framePlacer(parent, W, H);
  const imageAt = (url: string, name: string, b: Box): Layer => {
    const p = place.at(b.x, b.y);
    return {
      ...baseLayer(p.x, p.y), ...place.transform,
      kind: "image", name, product_id: null, url, width: b.w * place.kx, height: b.h * place.ky, fit: "stretch",
    };
  };

  ensureFontStylesheet();
  await Promise.all([`100px "${FONT}"`, `bold 100px "${FONT}"`].map((f) => document.fonts.load(f).catch(() => [])));
  const measure = blankCanvas(1, 1).ctx;
  const texts = lines.map((l, i): Layer => {
    const { color, bold } = styles[i];
    measure.font = `${bold ? "bold " : ""}100px "${FONT}"`;
    const at100 = measure.measureText(l.text).width;
    const boxW = l.box.x1 - l.box.x0;
    const size = fitFontSize(l.rowHeight, boxW, at100);
    const width = Math.max(boxW, (at100 * size) / 100) * 1.05;
    // Centre the line box on the ink, so the retyped line sits where the old one was.
    const p = place.at(l.box.x0, (l.box.y0 + l.box.y1) / 2 - size / 2);
    return {
      ...baseLayer(p.x, p.y), ...place.transform,
      kind: "text", text: l.text, fontFamily: FONT, fontSize: size * place.ky, fontStyle: bold ? "bold" : "normal",
      align: "left", width: width * place.kx, letterSpacing: 0, lineHeight: 1, textDecoration: "none",
      fill: { type: "solid", color },
    };
  });

  return [
    imageAt(bgUrl, "Background", { x: 0, y: 0, w: W, h: H }),
    ...cutouts.map((p, i) => imageAt(pieceUrls[i], `Object ${i + 1}`, p.box)),
    ...texts,
  ];
}
