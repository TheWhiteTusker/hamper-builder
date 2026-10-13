import type { SupabaseClient } from "@supabase/supabase-js";
import { calculateExpiry } from "@/lib/bin";
import type { BinItem } from "./bin-types";

export async function loadBinItems(supabase: SupabaseClient): Promise<BinItem[]> {
  const [
    { data: hampers },
    { data: products },
    { data: images },
  ] = await Promise.all([
    supabase
      .from("hampers")
      .select("id, code, name, collection, status, final_catalogue_sp, image_url, deleted_at")
      .not("deleted_at", "is", null)
      .order("deleted_at", { ascending: false }),
    supabase
      .from("products")
      .select("id, code, name, default_sp, cost_price, image_url, deleted_at, categories(name)")
      .not("deleted_at", "is", null)
      .order("deleted_at", { ascending: false }),
    supabase
      .from("product_images")
      .select("id, product_id, url, storage_path, color, is_primary, deleted_at, products(name, code)")
      .not("deleted_at", "is", null)
      .order("deleted_at", { ascending: false }),
  ]);

  const items: BinItem[] = [];

  for (const h of hampers ?? []) {
    if (!h.deleted_at) continue;
    const exp = calculateExpiry(h.deleted_at);
    items.push({
      id: h.id,
      type: "hamper",
      title: h.name,
      code: h.code,
      subtitle: `${h.collection ? `${h.collection} • ` : ""}${h.status} • SP: ₹${h.final_catalogue_sp ?? "—"}`,
      imageUrl: h.image_url,
      deletedAt: h.deleted_at,
      expiresAt: exp.expiresAt,
      daysRemaining: exp.daysRemaining,
      metadata: {
        collection: h.collection,
        status: h.status,
        finalCatalogueSp: h.final_catalogue_sp,
      },
    });
  }

  for (const p of products ?? []) {
    if (!p.deleted_at) continue;
    const exp = calculateExpiry(p.deleted_at);
    const catName = (p.categories as unknown as { name?: string } | null)?.name;
    items.push({
      id: p.id,
      type: "product",
      title: p.name,
      code: p.code,
      subtitle: `${catName ? `${catName} • ` : ""}SP: ₹${p.default_sp} • Cost: ₹${p.cost_price}`,
      imageUrl: p.image_url,
      deletedAt: p.deleted_at,
      expiresAt: exp.expiresAt,
      daysRemaining: exp.daysRemaining,
      metadata: {
        categoryName: catName,
        defaultSp: p.default_sp,
        costPrice: p.cost_price,
      },
    });
  }

  for (const img of images ?? []) {
    if (!img.deleted_at) continue;
    const exp = calculateExpiry(img.deleted_at);
    const prod = img.products as unknown as { name?: string; code?: string } | null;
    items.push({
      id: img.id,
      type: "image",
      title: prod?.name ? `Photo for ${prod.name}` : "Product Photo",
      code: prod?.code ?? null,
      subtitle: `Product: ${prod?.name || "Unknown"} (${prod?.code || ""})${img.color ? ` • Color: ${img.color}` : ""}`,
      imageUrl: img.url,
      deletedAt: img.deleted_at,
      expiresAt: exp.expiresAt,
      daysRemaining: exp.daysRemaining,
      metadata: {
        productId: img.product_id,
        productName: prod?.name,
        productCode: prod?.code,
        color: img.color,
        storagePath: img.storage_path,
      },
    });
  }

  // Sort by deletion date descending (newest deletions first)
  items.sort((a, b) => new Date(b.deletedAt).getTime() - new Date(a.deletedAt).getTime());

  return items;
}
