"use server";

import { z } from "zod";
import { createClient } from "@/lib/supabase/server";
import { type DeckItemInfo, type HamperInfo, type Photo, type ProductInfo } from "@/lib/presentation";
import { getBrandLogo } from "../brand-actions";
import { ItemSchema, type DeckItem } from "./deck-schemas";

export type DeckData = {
  hampers: HamperInfo[];
  products: ProductInfo[];
  logo: Photo | null;
  error?: string;
};

/* ------------------------------------------------------------- deck data */

type HamperRow = { id: string; code: string; name: string; final_catalogue_sp: number | null; image_url: string | null };
type LineRow = {
  hamper_id: string;
  product_id: string | null;
  product_name: string;
  category_name: string | null;
  products: { image_url: string | null } | null;
};
type ProductRow = {
  id: string;
  code: string;
  name: string;
  default_sp: number | null;
  image_url: string | null;
  categories: { name: string } | null;
};

/**
 * Everything the browser needs to lay out slides for the picked hampers and
 * products. Packaging that doesn't count as an item (ribbons, fillers) stays
 * off the slides, as in the printed deck.
 */
export async function loadDeckData(items: DeckItem[]): Promise<DeckData> {
  const parsed = z.array(ItemSchema).safeParse(items);
  if (!parsed.success) return { hampers: [], products: [], logo: null, error: "Invalid selection." };
  const supabase = await createClient();
  const hamperIds = parsed.data.filter((i) => i.type === "hamper").map((i) => i.id);
  const productIds = parsed.data.filter((i) => i.type === "product").map((i) => i.id);

  const [hampers, lines, products, categories, logo] = await Promise.all([
    hamperIds.length
      ? supabase
          .from("hamper_summary")
          .select("id, code, name, final_catalogue_sp, image_url")
          .in("id", hamperIds)
          .overrideTypes<HamperRow[], { merge: false }>()
          .then((r) => r.data ?? [])
      : Promise.resolve([] as HamperRow[]),
    hamperIds.length
      ? supabase
          .from("hamper_items")
          .select("hamper_id, product_id, product_name, category_name, products(image_url)")
          .in("hamper_id", hamperIds)
          .order("line_no")
          .overrideTypes<LineRow[], { merge: false }>()
          .then((r) => r.data ?? [])
      : Promise.resolve([] as LineRow[]),
    productIds.length
      ? supabase
          .from("products")
          .select("id, code, name, default_sp, image_url, categories(name)")
          .in("id", productIds)
          .overrideTypes<ProductRow[], { merge: false }>()
          .then((r) => r.data ?? [])
      : Promise.resolve([] as ProductRow[]),
    supabase
      .from("categories")
      .select("name, counts_as_item")
      .overrideTypes<{ name: string; counts_as_item: boolean }[], { merge: false }>()
      .then((r) => r.data ?? []),
    getBrandLogo(),
  ]);

  const counts = new Map(categories.map((c) => [c.name.trim().toLowerCase(), c.counts_as_item]));
  const countsAsItem = (category: string | null) => counts.get((category ?? "").trim().toLowerCase()) ?? true;

  return {
    logo,
    hampers: hamperIds.flatMap((id): HamperInfo[] => {
      const h = hampers.find((x) => x.id === id);
      if (!h) return [];
      const items: DeckItemInfo[] = lines
        .filter((l) => l.hamper_id === id && countsAsItem(l.category_name))
        .map((l) => ({
          product_id: l.product_id,
          name: l.product_name,
          caption: l.category_name ?? "",
          image_url: l.products?.image_url ?? null,
        }));
      return [
        {
          id: h.id,
          code: h.code,
          name: h.name,
          price: h.final_catalogue_sp == null ? null : Number(h.final_catalogue_sp),
          image_url: h.image_url,
          items,
        },
      ];
    }),
    products: productIds.flatMap((id): ProductInfo[] => {
      const p = products.find((x) => x.id === id);
      if (!p) return [];
      return [
        {
          id: p.id,
          code: p.code,
          name: p.name,
          caption: p.categories?.name ?? "",
          price: p.default_sp == null ? null : Number(p.default_sp),
          image_url: p.image_url,
        },
      ];
    }),
  };
}
