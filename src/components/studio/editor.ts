import type {
  HamperCanvas,
  Layer,
  LayerMove,
  PageAlign,
} from "@/lib/hamper-canvas";

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

const style = { fill: { type: "solid" as const, color: "#c8a97e" }, stroke: "#2c332f", strokeWidth: 0 };

export const SHAPES: { label: string; svg: string; make: (page: HamperCanvas) => Layer }[] = [
  {
    label: "Square",
    svg: '<rect x="4" y="4" width="40" height="40"/>',
    make: (p) => ({ ...baseLayer(p.width / 2 - 150, p.height / 2 - 150), ...style, kind: "rect", width: 300, height: 300, cornerRadius: 0 }),
  },
  {
    label: "Rounded square",
    svg: '<rect x="4" y="4" width="40" height="40" rx="10"/>',
    make: (p) => ({ ...baseLayer(p.width / 2 - 150, p.height / 2 - 150), ...style, kind: "rect", width: 300, height: 300, cornerRadius: 48 }),
  },
  {
    label: "Rectangle",
    svg: '<rect x="2" y="12" width="44" height="24"/>',
    make: (p) => ({ ...baseLayer(p.width / 2 - 250, p.height / 2 - 100), ...style, kind: "rect", width: 500, height: 200, cornerRadius: 0 }),
  },
  {
    label: "Circle",
    svg: '<circle cx="24" cy="24" r="20"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "ellipse", radiusX: 150, radiusY: 150 }),
  },
  {
    label: "Triangle",
    svg: '<polygon points="24,4 45,42 3,42"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "polygon", sides: 3, radius: 170 }),
  },
  {
    label: "Pentagon",
    svg: '<polygon points="24,3 45,19 37,44 11,44 3,19"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "polygon", sides: 5, radius: 160 }),
  },
  {
    label: "Hexagon",
    svg: '<polygon points="24,3 43,14 43,34 24,45 5,34 5,14"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "polygon", sides: 6, radius: 160 }),
  },
  {
    label: "Star",
    svg: '<polygon points="24,3 30,18 46,18 33,28 38,44 24,34 10,44 15,28 2,18 18,18"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "star", numPoints: 5, innerRadius: 70, outerRadius: 170 }),
  },
  {
    label: "Burst",
    svg: '<polygon points="24,2 28,15 40,8 33,20 46,24 33,28 40,40 28,33 24,46 20,33 8,40 15,28 2,24 15,20 8,8 20,15"/>',
    make: (p) => ({ ...baseLayer(p.width / 2, p.height / 2), ...style, kind: "star", numPoints: 12, innerRadius: 130, outerRadius: 170 }),
  },
  {
    label: "Line",
    svg: '<line x1="6" y1="24" x2="42" y2="24" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>',
    make: (p) => ({
      ...baseLayer(p.width / 2 - 200, p.height / 2),
      kind: "line",
      points: [0, 0, 400, 0],
      stroke: "#2c332f",
      strokeWidth: 4,
      lineCap: "round",
    }),
  },
  {
    label: "Curved line",
    svg: '<path d="M6 34 Q24 10 42 34" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>',
    make: (p) => ({
      ...baseLayer(p.width / 2 - 200, p.height / 2),
      kind: "curve",
      points: [0, 0, 200, -120, 400, 0],
      curvature: 0.5,
      stroke: "#2c332f",
      strokeWidth: 4,
      lineCap: "round",
    }),
  },
  {
    label: "Dashed line",
    svg: '<line x1="6" y1="24" x2="42" y2="24" stroke="currentColor" stroke-width="4" stroke-linecap="round" stroke-dasharray="6,4"/>',
    make: (p) => ({
      ...baseLayer(p.width / 2 - 200, p.height / 2),
      kind: "line",
      points: [0, 0, 400, 0],
      stroke: "#2c332f",
      strokeWidth: 4,
      lineCap: "round",
      dash: [16, 12],
    }),
  },
];

