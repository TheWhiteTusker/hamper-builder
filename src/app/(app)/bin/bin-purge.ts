"use server";

import { revalidatePath } from "next/cache";
import { requireUser, canManage } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { thumbPath } from "@/lib/product-images";
import type { BinActionResult } from "./bin-types";

const BUCKET = "product-images";

/**
 * Purges any items older than 30 days.
 */
export async function purgeExpiredAction(): Promise<BinActionResult> {
  try {
    const { supabase, profile } = await requireUser();
    if (!canManage(profile.role)) return { error: "Permission denied." };

    const thirtyDaysAgo = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();

    const { data: expImages } = await supabase
      .from("product_images")
      .select("id, storage_path")
      .not("deleted_at", "is", null)
      .lte("deleted_at", thirtyDaysAgo);

    for (const img of expImages ?? []) {
      if (img.storage_path) await supabase.storage.from(BUCKET).remove([img.storage_path, thumbPath(img.storage_path)]);
    }

    const { data: expHampers } = await supabase
      .from("hampers")
      .select("id")
      .not("deleted_at", "is", null)
      .lte("deleted_at", thirtyDaysAgo);

    for (const h of expHampers ?? []) {
      const { data: files } = await supabase.storage.from(BUCKET).list(`hampers/${h.id}`);
      if (files?.length) await supabase.storage.from(BUCKET).remove(files.map((f) => `hampers/${h.id}/${f.name}`));
    }

    const { data: expProducts } = await supabase
      .from("products")
      .select("id")
      .not("deleted_at", "is", null)
      .lte("deleted_at", thirtyDaysAgo);

    for (const p of expProducts ?? []) {
      const { data: files } = await supabase.storage.from(BUCKET).list(`products/${p.id}`);
      if (files?.length) await supabase.storage.from(BUCKET).remove(files.map((f) => `products/${p.id}/${f.name}`));
    }

    await supabase.rpc("purge_expired_bin_records");

    revalidatePath("/hampers");
    revalidatePath("/products");
    revalidatePath("/bin");
    return { ok: true, message: "Expired items have been permanently purged." };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}
