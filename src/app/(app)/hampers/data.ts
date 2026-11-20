import type { SupabaseClient } from "@supabase/supabase-js";
import { loadSettings } from "@/lib/settings";
import type { Category } from "@/lib/types";
import type { CatalogProduct } from "./hamper-builder";

type ProductRow = {
  id: string;
  code: string;
  name: string;
  category_id: string | null;
  source: string | null;
  cost_price: number;
  markup_pct: number;
  target_margin: number;
  default_sp: number;
  colors: string[];
  image_url: string | null;
  categories: { name: string; counts_as_item: boolean } | null;
};

/**
 * Everything the builder needs to work offline in the browser: the active
 * catalogue, the categories, and the pick-lists. Loading it up front is what
 * removes the round-trip the spreadsheet made on every category change.
 */
export async function loadCatalog(supabase: SupabaseClient) {
  const [{ data: products }, { data: categories }, settings] = await Promise.all([
    supabase
      .from("products")
      .select(
        "id, code, name, category_id, source, cost_price, markup_pct, target_margin, default_sp, colors, image_url, categories(name, counts_as_item)",
      )
      .eq("is_active", true)
      .is("deleted_at", null)
      .order("name")
      .overrideTypes<ProductRow[], { merge: false }>(),
    supabase
      .from("categories")
      .select("*")
      .order("sort_order")
      .order("name")
      .overrideTypes<Category[], { merge: false }>(),
    loadSettings(supabase),
  ]);

  const catalog: CatalogProduct[] = (products ?? []).map((p) => ({
    id: p.id,
    code: p.code,
    name: p.name,
    category_id: p.category_id,
    category_name: p.categories?.name ?? null,
    counts_as_item: p.categories?.counts_as_item ?? true,
    source: p.source,
    cost_price: Number(p.cost_price),
    markup_pct: Number(p.markup_pct ?? 0),
    target_margin: Number(p.target_margin),
    default_sp: Number(p.default_sp),
    colors: p.colors ?? [],
    image_url: p.image_url ?? null,
  }));

  return { products: catalog, categories: categories ?? [], settings };
}
