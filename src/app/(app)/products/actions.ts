"use server";

import { z } from "zod";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createClient, requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { codesForColors, resolveColors } from "@/lib/product-code";
import { productHref } from "./href";
import { roundUpToNext10 } from "@/lib/numbers.ts";
import {
  type ActionState,
  optionalText,
  money,
  checkbox,
  describeError,
} from "@/lib/forms";

const ProductSchema = z.object({
  id: z.string().uuid().optional().or(z.literal("").transform(() => undefined)),
  code: z.string().trim().min(1, "Product code is required"),
  name: z.string().trim().min(1, "Product name is required"),
  category_id: z
    .string()
    .transform((v) => (v === "" ? null : v))
    .nullable(),
  source: optionalText,
  cost_price: money,
  markup_pct: z.coerce.number().default(0),
  default_sp: money,
  colors: z.array(z.string()).default([]),
  is_active: checkbox,
});

export async function saveProduct(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const rawData: Record<string, unknown> = Object.fromEntries(formData);
  rawData.colors = formData.getAll("colors").map(String).filter(Boolean);

  const parsed = ProductSchema.safeParse(rawData);

  if (!parsed.success) {
    return { error: parsed.error.issues[0].message };
  }

  const { id, ...fields } = parsed.data;
  // Selling prices round off to the next 10 (e.g. 271.50 -> 280)
  const default_sp = roundUpToNext10(fields.default_sp);
  // Margin is no longer entered on the form; it is still stored because hamper
  // costing snapshots it, so keep it in step with the price actually set.
  const values = {
    ...fields,
    default_sp,
    target_margin: default_sp > 0 ? Math.round(((default_sp - fields.cost_price) / default_sp) * 10000) / 10000 : 0,
  };
  const supabase = await createClient();

  // A new product in several colours becomes one row per colour, each with its
  // own code (LC/0001/WL, LC/0001/BL). Editing an existing product stays one row.
  const rows =
    id || values.colors.length < 2
      ? [values]
      : codesForColors(values.code, values.colors).map(({ color, code }) => ({
          ...values,
          code,
          colors: [color],
        }));

  if (id) {
    const { error } = await supabase
      .from("products")
      .update({ ...values, deleted_at: null })
      .eq("id", id);
    if (error) return { error: describeError(error) };
  } else {
    for (const r of rows) {
      const { data: existing } = await supabase
        .from("products")
        .select("id")
        .eq("code", r.code)
        .maybeSingle();

      if (existing) {
        const { error } = await supabase
          .from("products")
          .update({ ...r, deleted_at: null })
          .eq("id", existing.id);
        if (error) return { error: describeError(error) };
      } else {
        const { error } = await supabase
          .from("products")
          .insert({ ...r, deleted_at: null });
        if (error) return { error: describeError(error) };
      }
    }
  }

  const targetCode = rows[0]?.code ?? values.code;
  revalidatePath("/products");
  for (const r of rows) {
    revalidatePath(`/products/${encodeURIComponent(r.code)}`);
  }
  redirect(`/products/${encodeURIComponent(targetCode)}`);
}

export async function deleteProduct(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const id = String(formData.get("id") ?? "");
  if (!id) return { error: "Missing product." };

  const supabase = await createClient();
  const { error } = await supabase
    .from("products")
    .update({ deleted_at: new Date().toISOString() })
    .eq("id", id);

  if (error) {
    return { error: describeError(error) };
  }

  revalidatePath("/products");
  revalidatePath("/bin");
  redirect("/products?deleted=1");
}

export async function toggleProductActive(formData: FormData) {
  const id = String(formData.get("id") ?? "");
  const next = String(formData.get("next") ?? "") === "true";

  const supabase = await createClient();
  await supabase.from("products").update({ is_active: next }).eq("id", id);

  revalidatePath("/products");
}

/**
 * Changes one product's colour. Its code follows (ED/0006/WL -> ED/0006/NE);
 * recolor_product updates the cost sheet, hamper lines and photo tags with it.
 */
export async function recolorProduct(_prev: ActionState, formData: FormData): Promise<ActionState> {
  const { supabase } = await requireRole("admin");
  const id = String(formData.get("id") ?? "");
  const name = String(formData.get("color") ?? "");
  const settings = await loadSettings(supabase);
  const color = resolveColors(settings.product_colors, settings.color_hex).find((c) => c.name === name);
  if (!id || !color) return { error: "Pick a colour." };

  const { data: code, error } = await supabase.rpc("recolor_product", {
    p_id: id,
    p_color: color.name,
    p_color_code: color.code,
  });
  if (error) return { error: describeError(error) };

  revalidatePath("/products", "layout");
  revalidatePath("/cost-calculator", "layout");
  revalidatePath("/hampers", "layout");
  redirect(productHref(String(code)));
}
