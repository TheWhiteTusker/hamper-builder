import type { SupabaseClient } from "@supabase/supabase-js";
import type {
  CostCategory,
  CostStage,
  CostStageWithHierarchy,
  CostSubcategory,
  CostVariety,
  Product,
  ProductCostLine,
  ProductCostSheet,
  ProductImage,
} from "@/lib/types";
import { parseProductCode } from "@/lib/product-code";
import { variantIdsByColor } from "@/lib/product-variants";
import { buildCostTree, type CostRows } from "@/lib/cost-tree";

const active = <T,>(supabase: SupabaseClient, table: string) =>
  supabase.from(table).select("*").eq("is_active", true).order("sort_order").returns<T[]>();

/** The costing hierarchy's four tables, flat (active rows only). */
export async function loadCostRows(supabase: SupabaseClient): Promise<CostRows> {
  const [{ data: stages }, { data: categories }, { data: subcategories }, { data: varieties }] =
    await Promise.all([
      active<CostStage>(supabase, "cost_stages"),
      active<CostCategory>(supabase, "cost_categories"),
      active<CostSubcategory>(supabase, "cost_subcategories"),
      active<CostVariety>(supabase, "cost_varieties"),
    ]);
  return {
    stages: stages ?? [],
    categories: categories ?? [],
    subcategories: subcategories ?? [],
    varieties: varieties ?? [],
  };
}

/** The 4-level costing hierarchy: stage -> category -> subcategory -> variety. */
export async function loadCostStages(supabase: SupabaseClient): Promise<CostStageWithHierarchy[]> {
  return buildCostTree(await loadCostRows(supabase));
}

/** A product with its cost sheet (and lines) and photos, or null if the code is unknown. */
export async function loadProductCosting(supabase: SupabaseClient, code: string) {
  const { data: product } = await supabase
    .from("products")
    .select("*")
    .eq("code", code)
    .is("deleted_at", null)
    .maybeSingle<Product>();
  if (!product) return null;

  const { categoryCode, serial } = parseProductCode(product.code);
  let siblingColors: string[] = product.colors ?? [];
  let allProductIds: string[] = [product.id];
  let siblingIds: string[] = [];
  let familyProducts: { id: string; code: string; colors?: string[] | null }[] = [product];

  if (categoryCode && serial) {
    const { data: siblings } = await supabase
      .from("products")
      .select("id, code, colors")
      .ilike("code", `${categoryCode}/${serial}/%`)
      .is("deleted_at", null);

    if (siblings && siblings.length > 0) {
      familyProducts = [product, ...siblings.filter((s) => s.id !== product.id)];
      siblingIds = siblings.map((s) => s.id);
      allProductIds = Array.from(new Set([product.id, ...siblingIds]));
      const collected = new Set<string>(product.colors ?? []);
      for (const s of siblings) {
        for (const col of s.colors ?? []) {
          collected.add(col);
        }
      }
      siblingColors = Array.from(collected);
    }
  }

  const [{ data: ownSheet }, { data: images }] = await Promise.all([
    supabase
      .from("product_cost_sheets")
      .select("*")
      .eq("product_id", product.id)
      .maybeSingle<ProductCostSheet>(),
    supabase
      .from("product_images")
      .select("*")
      .in("product_id", allProductIds)
      .is("deleted_at", null)
      .order("sort_order")
      .order("created_at")
      .returns<ProductImage[]>(),
  ]);

  let sheet: ProductCostSheet | null = ownSheet;
  if (!sheet && siblingIds.length > 0) {
    const { data: altSheet } = await supabase
      .from("product_cost_sheets")
      .select("*")
      .in("product_id", siblingIds)
      .order("created_at", { ascending: false })
      .limit(1)
      .maybeSingle<ProductCostSheet>();
    sheet = altSheet;
  }

  let lines: ProductCostLine[] = [];
  if (sheet) {
    const { data } = await supabase
      .from("product_cost_lines")
      .select("*")
      .eq("sheet_id", sheet.id)
      .order("sort_order")
      .returns<ProductCostLine[]>();
    lines = data ?? [];
  }

  return {
    product: { ...product, colors: siblingColors.length > 0 ? siblingColors : product.colors },
    sheet: sheet ? { ...sheet, lines } : null,
    images: images ?? [],
    variantIds: variantIdsByColor(familyProducts),
  };
}
