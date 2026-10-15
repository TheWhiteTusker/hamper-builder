import type { HamperCanvas, Layer } from "@/lib/hamper-canvas";
import { baseLayer } from "./editor";

const style = { fill: { type: "solid" as const, color: "#c8a97e" }, stroke: "#2c332f", strokeWidth: 0 };

/** The Elements panel's shapes and lines, each made at the centre of the page. */
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
