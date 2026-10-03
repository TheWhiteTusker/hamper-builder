"use server";

import { revalidatePath } from "next/cache";
import { requireUser, canManage, isAdmin } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { thumbPath, viewPath } from "@/lib/product-images";
import type { BinItemType, BinActionResult } from "./bin-types";

const BUCKET = "product-images";

/**
 * Permanently deletes a single item from the database and removes any associated storage files.
 */
export async function permanentlyDeleteBinItem(
  id: string,
  type: BinItemType,
): Promise<BinActionResult> {
  try {
    const { supabase, profile } = await requireUser();

    if (type === "hamper") {
      if (!canManage(profile.role)) {
        return { error: "You don't have permission to permanently delete hampers." };
      }

      const { data: files } = await supabase.storage.from(BUCKET).list(`hampers/${id}`);
      if (files?.length) {
        await supabase.storage.from(BUCKET).remove(files.map((f) => `hampers/${id}/${f.name}`));
      }

      const { error: delErr } = await supabase.from("hampers").delete().eq("id", id);
      if (delErr) return { error: describeError(delErr) };

      revalidatePath("/hampers");
      revalidatePath("/bin");
      return { ok: true, message: "Hamper permanently deleted." };
    }

    if (type === "product") {
      if (!isAdmin(profile.role)) {
        return { error: "Only administrators can permanently delete products." };
      }

      const { data: images } = await supabase
        .from("product_images")
        .select("storage_path")
        .eq("product_id", id);

      const paths = (images ?? []).map((img) => img.storage_path).filter((p): p is string => Boolean(p));
      if (paths.length) {
        await supabase.storage.from(BUCKET).remove(paths);
      }

      const { data: files } = await supabase.storage.from(BUCKET).list(`products/${id}`);
      if (files?.length) {
        await supabase.storage.from(BUCKET).remove(files.map((f) => `products/${id}/${f.name}`));
      }

      await supabase.from("product_images").delete().eq("product_id", id);
      const { error: delErr } = await supabase.from("products").delete().eq("id", id);
      if (delErr) return { error: describeError(delErr) };

      revalidatePath("/products");
      revalidatePath("/bin");
      return { ok: true, message: "Product permanently deleted." };
    }

    if (type === "image") {
      if (!canManage(profile.role)) {
        return { error: "You don't have permission to delete photos." };
      }

      const { data: img } = await supabase
        .from("product_images")
        .select("storage_path")
        .eq("id", id)
        .maybeSingle<{ storage_path: string | null }>();

      if (img?.storage_path) {
        await supabase.storage.from(BUCKET).remove([img.storage_path, thumbPath(img.storage_path), viewPath(img.storage_path)]);
      }

      const { error: delErr } = await supabase.from("product_images").delete().eq("id", id);
      if (delErr) return { error: describeError(delErr) };

      revalidatePath("/products");
      revalidatePath("/bin");
      return { ok: true, message: "Photo permanently deleted." };
    }

    return { error: "Unknown item type." };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

/**
 * Permanently deletes all items in the Bin, or all items of a specific type.
 */
export async function emptyBinAction(
  scope: "all" | BinItemType = "all",
): Promise<BinActionResult> {
  try {
    const { supabase, profile } = await requireUser();
    let deletedCount = 0;

    // Hampers
    if (scope === "all" || scope === "hamper") {
      if (canManage(profile.role)) {
        const { data: hampers } = await supabase
          .from("hampers")
          .select("id")
          .not("deleted_at", "is", null);

        for (const h of hampers ?? []) {
          const { data: files } = await supabase.storage.from(BUCKET).list(`hampers/${h.id}`);
          if (files?.length) {
            await supabase.storage.from(BUCKET).remove(files.map((f) => `hampers/${h.id}/${f.name}`));
          }
          await supabase.from("hampers").delete().eq("id", h.id);
          deletedCount++;
        }
      }
    }

    // Products
    if (scope === "all" || scope === "product") {
      if (isAdmin(profile.role)) {
        const { data: products } = await supabase
          .from("products")
          .select("id")
          .not("deleted_at", "is", null);

        for (const p of products ?? []) {
          const { data: images } = await supabase
            .from("product_images")
            .select("storage_path")
            .eq("product_id", p.id);

          const paths = (images ?? []).map((img) => img.storage_path).filter((s): s is string => Boolean(s));
          if (paths.length) {
            await supabase.storage.from(BUCKET).remove(paths);
          }

          const { data: files } = await supabase.storage.from(BUCKET).list(`products/${p.id}`);
          if (files?.length) {
            await supabase.storage.from(BUCKET).remove(files.map((f) => `products/${p.id}/${f.name}`));
          }

          await supabase.from("product_images").delete().eq("product_id", p.id);
          await supabase.from("products").delete().eq("id", p.id);
          deletedCount++;
        }
      }
    }

    // Product Images
    if (scope === "all" || scope === "image") {
      if (canManage(profile.role)) {
        const { data: images } = await supabase
          .from("product_images")
          .select("id, storage_path")
          .not("deleted_at", "is", null);

        for (const img of images ?? []) {
          if (img.storage_path) {
            await supabase.storage.from(BUCKET).remove([img.storage_path, thumbPath(img.storage_path), viewPath(img.storage_path)]);
          }
          await supabase.from("product_images").delete().eq("id", img.id);
          deletedCount++;
        }
      }
    }

    revalidatePath("/hampers");
    revalidatePath("/products");
    revalidatePath("/bin");
    return { ok: true, message: `Successfully deleted ${deletedCount} item${deletedCount === 1 ? "" : "s"} permanently.` };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

