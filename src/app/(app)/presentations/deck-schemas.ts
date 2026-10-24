import { z } from "zod";
import { CanvasSchema } from "@/lib/hamper-canvas-schema";

/** What the server actions accept for picked items and slides. */
export const ItemSchema = z.object({ type: z.enum(["hamper", "product"]), id: z.string().uuid() });
export type DeckItem = z.infer<typeof ItemSchema>;

export const SlideSchema = z.object({
  kind: z.enum(["cover", "hamper", "product", "closing", "blank"]),
  hamper_id: z.string().uuid().nullable(),
  product_id: z.string().uuid().nullable(),
  canvas: CanvasSchema,
});
export type SlideInput = z.input<typeof SlideSchema>;

/** An existing slide is sent by id; a slide added on the page since the last save is sent in full. */
export const DeckSlideSchema = z.union([
  z.object({ id: z.string().uuid() }).strict(),
  SlideSchema.extend({ tempId: z.string().min(1) }),
]);
export type DeckSlideInput = z.input<typeof DeckSlideSchema>;
