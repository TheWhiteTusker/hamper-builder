"use server";

// No revalidatePath: BinView drops the rows itself, and other pages refresh
// on the next navigation (markOtherPagesStale).

import { requireUser, canManage, isAdmin } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { BinItemType, BinActionResult } from "./bin-types";

/**
 * Restores a soft-deleted item from the Bin back into active use.
 */
export async function restoreBinItem(
  id: string,
  type: BinItemType,
): Promise<BinActionResult> {
  try {
    const { supabase, profile } = await requireUser();

    if (type === "hamper") {
      if (!canManage(profile.role)) {
        return { error: "You don't have permission to restore hampers." };
      }

      const { data: hamper, error: fetchErr } = await supabase
        .from("hampers")
        .select("id, code, name")
        .eq("id", id)
        .not("deleted_at", "is", null)
        .maybeSingle<{ id: string; code: string; name: string }>();

      if (fetchErr || !hamper) {
        return { error: "Hamper not found in Bin." };
      }

      // Check if an active hamper already has this code
      const { data: conflict } = await supabase
        .from("hampers")
        .select("id")
        .eq("code", hamper.code)
        .is("deleted_at", null)
        .maybeSingle();

      if (conflict) {
        return {
          error: `Cannot restore: an active hamper with code "${hamper.code}" already exists. Please rename or delete it first.`,
        };
      }

      const { error: updateErr } = await supabase
        .from("hampers")
        .update({ deleted_at: null })
        .eq("id", id);

      if (updateErr) return { error: describeError(updateErr) };

      return { ok: true, message: `Hamper "${hamper.name}" (${hamper.code}) restored.` };
    }

    if (type === "product") {
      if (!isAdmin(profile.role)) {
        return { error: "Only administrators can restore products." };
      }

      const { data: product, error: fetchErr } = await supabase
        .from("products")
        .select("id, code, name")
        .eq("id", id)
        .not("deleted_at", "is", null)
        .maybeSingle<{ id: string; code: string; name: string }>();

      if (fetchErr || !product) {
        return { error: "Product not found in Bin." };
      }

      // Check if an active product already has this code
      const { data: conflict } = await supabase
        .from("products")
        .select("id")
        .eq("code", product.code)
        .is("deleted_at", null)
        .maybeSingle();

      if (conflict) {
        return {
          error: `Cannot restore: an active product with code "${product.code}" already exists.`,
        };
      }

      const { error: updateErr } = await supabase
        .from("products")
        .update({ deleted_at: null })
        .eq("id", id);

      if (updateErr) return { error: describeError(updateErr) };

      return { ok: true, message: `Product "${product.name}" (${product.code}) restored.` };
    }

    if (type === "image") {
      if (!canManage(profile.role)) {
        return { error: "You don't have permission to restore photos." };
      }

      const { data: image, error: fetchErr } = await supabase
        .from("product_images")
        .select("id, product_id, url")
        .eq("id", id)
        .not("deleted_at", "is", null)
        .maybeSingle<{ id: string; product_id: string; url: string }>();

      if (fetchErr || !image) {
        return { error: "Photo not found in Bin." };
      }

      const { error: updateErr } = await supabase
        .from("product_images")
        .update({ deleted_at: null })
        .eq("id", id);

      if (updateErr) return { error: describeError(updateErr) };

      // If parent product currently has no image, set this as primary
      const { data: prod } = await supabase
        .from("products")
        .select("image_url")
        .eq("id", image.product_id)
        .maybeSingle<{ image_url: string | null }>();

      if (prod && !prod.image_url) {
        await supabase
          .from("products")
          .update({ image_url: image.url })
          .eq("id", image.product_id);
        await supabase
          .from("product_images")
          .update({ is_primary: true })
          .eq("id", id);
      }

      return { ok: true, message: "Product photo restored." };
    }

    return { error: "Unknown item type." };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}
