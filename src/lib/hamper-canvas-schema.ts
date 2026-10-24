import { z } from "zod";
import { emptyCanvas } from "./hamper-canvas.ts";

// The zod half of the canvas document. Kept apart from hamper-canvas.ts so that
// client components using the helpers there do not bundle zod (~380 kB).

const color = z.string().regex(/^#[0-9a-fA-F]{6}$/);

export const FillSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("solid"), color }),
  z.object({ type: z.literal("linear"), from: color, to: color, angle: z.number() }),
]);

// visible/locked/name arrived after the first saved designs, hence the defaults.
const base = {
  id: z.string().min(1),
  name: z.string().optional(),
  visible: z.boolean().default(true),
  locked: z.boolean().default(false),
  x: z.number(),
  y: z.number(),
  rotation: z.number(),
  scaleX: z.number(),
  scaleY: z.number(),
  opacity: z.number().min(0).max(1),
};

const shapeStyle = {
  fill: FillSchema,
  stroke: color,
  strokeWidth: z.number().min(0),
};

export const LayerSchema = z.discriminatedUnion("kind", [
  z.object({
    ...base,
    kind: z.literal("image"),
    product_id: z.string().uuid().nullable(),
    url: z.string().url(),
    width: z.number().positive(),
    height: z.number().positive(),
    /** "contain" fits the photo inside the width×height frame without stretching. */
    fit: z.enum(["stretch", "contain"]).default("stretch"),
  }),
  z.object({
    ...base,
    kind: z.literal("text"),
    text: z.string(),
    fontSize: z.number().positive(),
    fontFamily: z.string(),
    fontStyle: z.enum(["normal", "bold", "italic", "italic bold"]),
    align: z.enum(["left", "center", "right"]),
    width: z.number().positive(),
    letterSpacing: z.number().default(0),
    lineHeight: z.number().positive().default(1),
    textDecoration: z.enum(["none", "underline"]).default("none"),
    fill: FillSchema,
  }),
  z.object({
    ...base,
    ...shapeStyle,
    kind: z.literal("rect"),
    width: z.number().positive(),
    height: z.number().positive(),
    cornerRadius: z.number().min(0),
  }),
  z.object({
    ...base,
    ...shapeStyle,
    kind: z.literal("ellipse"),
    radiusX: z.number().positive(),
    radiusY: z.number().positive(),
  }),
  z.object({
    ...base,
    ...shapeStyle,
    kind: z.literal("polygon"),
    sides: z.number().int().min(3).max(12),
    radius: z.number().positive(),
  }),
  z.object({
    ...base,
    ...shapeStyle,
    kind: z.literal("star"),
    numPoints: z.number().int().min(3).max(20),
    innerRadius: z.number().positive(),
    outerRadius: z.number().positive(),
  }),
  z.object({
    ...base,
    kind: z.literal("line"),
    points: z.array(z.number()),
    stroke: color,
    strokeWidth: z.number().min(1).default(4),
    dash: z.array(z.number()).optional(),
    lineCap: z.enum(["round", "butt", "square"]).default("round"),
  }),
  z.object({
    ...base,
    kind: z.literal("curve"),
    points: z.array(z.number()),
    stroke: color,
    strokeWidth: z.number().min(1).default(4),
    curvature: z.number().default(0.5),
    dash: z.array(z.number()).optional(),
    lineCap: z.enum(["round", "butt", "square"]).default("round"),
  }),
]);

export const CanvasSchema = z.object({
  width: z.number().positive(),
  height: z.number().positive(),
  background: z.object({ fill: FillSchema, image_url: z.string().url().nullable() }),
  layers: z.array(LayerSchema),
});

export type Fill = z.infer<typeof FillSchema>;
export type Layer = z.infer<typeof LayerSchema>;
export type LayerKind = Layer["kind"];
export type HamperCanvas = z.infer<typeof CanvasSchema>;

/** A saved canvas, or a blank one if there is none or it no longer parses. */
export function parseCanvas(raw: unknown): HamperCanvas {
  const parsed = CanvasSchema.safeParse(raw);
  return parsed.success ? parsed.data : emptyCanvas();
}
