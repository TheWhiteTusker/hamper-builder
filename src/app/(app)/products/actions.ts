"use server";

import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createClient, requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { resolveColors } from "@/lib/product-code";
import { productHref } from "./href";
import { type ActionState, describeError } from "@/lib/forms";

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
