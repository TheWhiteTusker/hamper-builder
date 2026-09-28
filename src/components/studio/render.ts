"use client";

import type Konva from "konva";
import { backgroundImageAttrs, fillProps, layerConfig, type HamperCanvas, type Layer } from "@/lib/hamper-canvas";

/* ------------------------------------------------------------------ fonts */

export const FONTS: { family: string; google?: string }[] = [
  { family: "Playfair Display", google: "Playfair+Display:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Cormorant Garamond", google: "Cormorant+Garamond:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Lora", google: "Lora:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Cinzel", google: "Cinzel:wght@400;700" },
  { family: "Montserrat", google: "Montserrat:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Poppins", google: "Poppins:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Inter", google: "Inter:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Questrial", google: "Questrial" },
  { family: "Cabin", google: "Cabin:ital,wght@0,400;0,700;1,400;1,700" },
  { family: "Oswald", google: "Oswald:wght@400;700" },
  { family: "Bebas Neue", google: "Bebas+Neue" },
  { family: "Great Vibes", google: "Great+Vibes" },
  { family: "Dancing Script", google: "Dancing+Script:wght@400;700" },
  { family: "Pacifico", google: "Pacifico" },
  { family: "Arial" },
  { family: "Georgia" },
  { family: "Times New Roman" },
  { family: "Courier New" },
];

const FONTS_HREF = `https://fonts.googleapis.com/css2?${FONTS.filter((f) => f.google)
  .map((f) => `family=${f.google}`)
  .join("&")}&display=swap`;

/** Adds the Google Fonts stylesheet once per page. */
export function ensureFontStylesheet() {
  if (document.querySelector("link[data-studio-fonts]")) return;
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = FONTS_HREF;
  link.dataset.studioFonts = "";
  document.head.appendChild(link);
}

/** CSS font shorthands for every text face a canvas uses, e.g. `bold 40px "Inter"`. */
export const fontFaces = (canvas: HamperCanvas) => [
  ...new Set(canvas.layers.flatMap((l) => (l.kind === "text" ? [`${l.fontStyle} 40px "${l.fontFamily}"`] : []))),
];

/** Canvas text doesn't trigger web-font downloads by itself, so ask for each face in use. */
export async function loadFonts(canvas: HamperCanvas) {
  ensureFontStylesheet();
  await Promise.all(fontFaces(canvas).map((f) => document.fonts.load(f).catch(() => [])));
}

/* ----------------------------------------------------------------- images */

export const loadImage = (url: string, timeoutMs = 15000) =>
  new Promise<HTMLImageElement>((resolve, reject) => {
    const img = new window.Image();
    // Only set crossOrigin for remote URLs; setting it on data: or blob: can cause browser CORS glitches
    if (!url.startsWith("blob:") && !url.startsWith("data:")) {
      img.crossOrigin = "anonymous";
    }
    const timer = setTimeout(() => {
      img.onload = null;
      img.onerror = null;
      reject(new Error(`Timed out loading image: ${url}`));
    }, timeoutMs);

    img.onload = () => {
      clearTimeout(timer);
      resolve(img);
    };
    img.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`Could not load ${url}`));
    };
    img.src = url;
  });

const imageCache = new Map<string, Promise<HTMLImageElement | null>>();
const cachedImage = (url: string) => {
  if (!imageCache.has(url)) imageCache.set(url, loadImage(url).catch(() => null));
  return imageCache.get(url)!;
};

/* ---------------------------------------------------------------- render */

export type RenderedText = { layer: Extract<Layer, { kind: "text" }>; height: number };

/**
 * Draws a canvas off-screen and returns it as a PNG data URL, plus each text
 * layer's laid-out height. `skipText` leaves text out of the picture so the
 * PowerPoint export can add it back as editable text boxes.
 */
export async function renderCanvas(
  canvas: HamperCanvas,
  { pixelRatio = 1, skipText = false }: { pixelRatio?: number; skipText?: boolean } = {},
): Promise<{ dataUrl: string; texts: RenderedText[] }> {
  const { default: K } = await import("konva");
  await loadFonts(canvas);

  const urls = [
    ...new Set([
      ...(canvas.background.image_url ? [canvas.background.image_url] : []),
      ...canvas.layers.flatMap((l) => (l.kind === "image" && l.visible ? [l.url] : [])),
    ]),
  ];
  const images = new Map(await Promise.all(urls.map(async (u) => [u, await cachedImage(u)] as const)));

  const stage = new K.Stage({ container: document.createElement("div"), width: canvas.width, height: canvas.height });
  try {
    const layer = new K.Layer();
    stage.add(layer);
    layer.add(new K.Rect({ width: canvas.width, height: canvas.height, ...fillProps(canvas.background.fill, canvas.width, canvas.height) }));
    const bg = canvas.background.image_url ? images.get(canvas.background.image_url) : null;
    if (bg) layer.add(new K.Image({ image: bg, ...backgroundImageAttrs(canvas, bg) }));

    const texts: RenderedText[] = [];
    for (const l of canvas.layers) {
      if (!l.visible) continue;
      const img = l.kind === "image" ? images.get(l.url) : undefined;
      if (l.kind === "image" && !img) continue; // unreachable image: leave a gap rather than fail the export
      const { shape, attrs } = layerConfig(l, img ?? undefined);
      const Shape = K[shape] as unknown as new (config: object) => Konva.Shape;
      const node = new Shape(img ? { ...attrs, image: img } : attrs);
      if (l.kind === "text") {
        texts.push({ layer: l, height: node.height() });
        if (skipText) {
          node.destroy();
          continue;
        }
      }
      layer.add(node);
    }
    layer.draw();
    return { dataUrl: stage.toDataURL({ pixelRatio, mimeType: "image/png" }), texts };
  } finally {
    stage.destroy();
  }
}
