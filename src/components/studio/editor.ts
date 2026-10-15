import type {
  HamperCanvas,
  Layer,
  LayerMove,
  PageAlign,
} from "@/lib/hamper-canvas";
import type { MagicStep } from "./magic/run-magic-layers";

export type PickerProduct = { id: string; code: string; name: string; image_url: string | null };

/** What the editor's save and upload server actions return. */
export type ActionResult = { ok?: boolean; error?: string; url?: string };

/** A signed URL to upload one image straight to storage, and its public URL once uploaded. */
export type UploadTicket = { error?: string; path?: string; token?: string; url?: string };

/** Everything the panels and toolbars can do to the design. Built in canvas-stage.tsx. */
export type Editor = {
  canvas: HamperCanvas;
  /** The selected layer when exactly one is selected. */
  selected: Layer | null;
  /** All selected layers, in z-order. */
  selection: Layer[];
  selectedIds: string[];
  documentColors: string[];
  select: (id: string | null) => void;
  selectMany: (ids: string[]) => void;
  toggleSelect: (id: string) => void;
  change: (fn: (c: HamperCanvas) => HamperCanvas, group?: string | null) => void;
  patch: (id: string, p: Partial<Layer>, group?: string | null) => void;
  patchEach: (ids: string[], p: Partial<Layer>, group?: string | null) => void;
  /** patch for continuous controls: one undo step per control. */
  tweak: (id: string, p: Partial<Layer>) => void;
  add: (layer: Layer) => void;
  remove: (ids: string[]) => void;
  duplicate: (ids: string[]) => void;
  move: (ids: string[], m: LayerMove) => void;
  reorder: (id: string, to: number) => void;
  /** One layer aligns to the page; several align to each other. */
  align: (ids: string[], where: PageAlign) => void;
  distribute: (ids: string[], axis: "x" | "y") => void;
  flip: (id: string, axis: "x" | "y") => void;
  fitToPage: (id: string) => void;
  /** Adds a product image, or replaces the image being swapped. `at` is a page point. */
  placeImage: (product: PickerProduct, url: string, at?: { x: number; y: number }) => void;
  swapId: string | null;
  startSwap: (id: string | null) => void;
  uploadBackground: (file: File) => void;
  /** Uploads images and adds each to the page as its own layer. */
  uploadImages: (files: File[]) => void;
  /** Adds an image that isn't a product photo (an upload, the logo). `at` is a page point. */
  addImage: (url: string, name: string, at?: { x: number; y: number }) => void;
  addLogo: () => void;
  uploads: { url: string; name: string }[];
  uploading: boolean;
  cutOut: (id: string, tolerance: number) => void;
  removing: boolean;
  /** Magic layers: splits an image into background, object cut-outs and editable text. */
  magicSplit: (id: string) => void;
  /** What Magic layers is doing right now, or null when idle. */
  magicStep: MagicStep | null;
  /** Per-computer switch, so slower machines never download the models. */
  magicEnabled: boolean;
  setMagicEnabled: (on: boolean) => void;
  activeTool: "select" | "line" | "curve";
  setActiveTool: (tool: "select" | "line" | "curve") => void;
  snapRightAngles: boolean;
  setSnapRightAngles: (snap: boolean | ((prev: boolean) => boolean)) => void;
};

export const DRAG_MIME = "application/x-hamper-image";

export const baseLayer = (x: number, y: number) => ({
  id: crypto.randomUUID(),
  visible: true,
  locked: false,
  x,
  y,
  rotation: 0,
  scaleX: 1,
  scaleY: 1,
  opacity: 1,
});

type TextStyle = { fontFamily: string; fontSize: number; fontStyle: "normal" | "bold" | "italic" | "italic bold"; color: string; letterSpacing?: number };

export const TEXT_PRESETS: { label: string; text: string; style: TextStyle; preview: number }[] = [
  { label: "Add a heading", text: "Add a heading", style: { fontFamily: "Playfair Display", fontSize: 96, fontStyle: "bold", color: "#2c332f" }, preview: 22 },
  { label: "Add a subheading", text: "Add a subheading", style: { fontFamily: "Montserrat", fontSize: 56, fontStyle: "bold", color: "#2c332f" }, preview: 16 },
  { label: "Add a little bit of body text", text: "Add a little bit of body text", style: { fontFamily: "Inter", fontSize: 34, fontStyle: "normal", color: "#2c332f" }, preview: 12 },
];

export const TEXT_COMBOS: { text: string; style: TextStyle }[] = [
  { text: "Happy Diwali", style: { fontFamily: "Great Vibes", fontSize: 130, fontStyle: "normal", color: "#b08d57" } },
  { text: "SEASON'S GREETINGS", style: { fontFamily: "Cinzel", fontSize: 64, fontStyle: "bold", color: "#2c332f", letterSpacing: 6 } },
  { text: "With love", style: { fontFamily: "Dancing Script", fontSize: 110, fontStyle: "bold", color: "#c0392b" } },
  { text: "THE GOURMET BOX", style: { fontFamily: "Bebas Neue", fontSize: 120, fontStyle: "normal", color: "#54655b", letterSpacing: 4 } },
  { text: "Thank you", style: { fontFamily: "Pacifico", fontSize: 96, fontStyle: "normal", color: "#ff66c4" } },
  { text: "Curated with care", style: { fontFamily: "Cormorant Garamond", fontSize: 72, fontStyle: "italic", color: "#2c332f" } },
];

export function makeText(page: HamperCanvas, text: string, s: TextStyle): Layer {
  const width = page.width * 0.8;
  return {
    ...baseLayer((page.width - width) / 2, page.height / 2 - s.fontSize / 2),
    kind: "text",
    text,
    fontFamily: s.fontFamily,
    fontSize: s.fontSize,
    fontStyle: s.fontStyle,
    align: "center",
    width,
    letterSpacing: s.letterSpacing ?? 0,
    lineHeight: 1.1,
    textDecoration: "none",
    fill: { type: "solid", color: s.color },
  };
}
