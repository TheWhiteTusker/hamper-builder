import type { SupabaseClient } from "@supabase/supabase-js";
import { describeError } from "@/lib/forms";
import { planReprice, type MasterRate, type PricedProduct } from "@/lib/reprice-plan";
import type { ProductCostLine, ProductCostSheet } from "@/lib/types";

type VarietyRow = {
  id: string;
  name: string;
  default_rate: number;
  unit: string;
  cost_subcategories: { name: string; cost_categories: { name: string } | null } | null;
};

/**
 * After master rates change: re-rate every saved line using those varieties,
 * recompute their cost sheets and reprice their products (see planReprice).
 * Quotations, proforma invoices and hampers keep their own saved prices.
 *
 * Reads and writes in bulk, a handful of requests however many products are
 * affected: a Worker invocation caps its outgoing requests.
 *
 * ponytail: several statements, not one transaction; move into an RPC if a
 * half-applied reprice (network drop mid-way) becomes a real problem.
 */
export async function repriceVarieties(
  supabase: SupabaseClient,
  varietyIds: string[],
): Promise<{ products: number; lines: number; sheets: number; error?: string }> {
  const none = { products: 0, lines: 0, sheets: 0 };
  if (!varietyIds.length) return none;

  const [{ data: varieties, error: vError }, { data: used, error: uError }] = await Promise.all([
    supabase
      .from("cost_varieties")
      .select("id, name, default_rate, unit, cost_subcategories(name, cost_categories(name))")
      .in("id", varietyIds)
      .returns<VarietyRow[]>(),
    supabase.from("product_cost_lines").select("sheet_id").in("cost_variety_id", varietyIds),
  ]);
  if (vError || uError) return { ...none, error: describeError(vError ?? uError) };
  const sheetIds = [...new Set((used ?? []).map((l: { sheet_id: string }) => l.sheet_id))];
  if (!sheetIds.length) return none;

  const [lines, sheets, products, costed] = await Promise.all([
    supabase.from("product_cost_lines").select("*").in("sheet_id", sheetIds).returns<ProductCostLine[]>(),
    supabase.from("product_cost_sheets").select("*").in("id", sheetIds).returns<ProductCostSheet[]>(),
    supabase.from("products").select("id, code, name, cost_price, default_sp").returns<PricedProduct[]>(),
    supabase.from("product_cost_sheets").select("product_id").not("product_id", "is", null),
  ]);
  const readError = lines.error ?? sheets.error ?? products.error ?? costed.error;
  if (readError) return { ...none, error: describeError(readError) };

  const rates: MasterRate[] = (varieties ?? []).map((v) => ({
    id: v.id,
    name: v.name,
    default_rate: v.default_rate,
    unit: v.unit,
    subcategory_name: v.cost_subcategories?.name ?? null,
    category_name: v.cost_subcategories?.cost_categories?.name ?? null,
  }));
  const plan = planReprice({
    rates,
    lines: lines.data ?? [],
    sheets: sheets.data ?? [],
    products: products.data ?? [],
    costedAlone: new Set((costed.data ?? []).map((s: { product_id: string }) => s.product_id)),
  });

  // Upsert on id updates the existing rows in one request per table.
  const writes = [
    plan.lines.length && (() => supabase.from("product_cost_lines").upsert(plan.lines, { onConflict: "id" })),
    plan.products.length && (() => supabase.from("products").upsert(plan.products, { onConflict: "id" })),
    plan.sheets.length && (() => supabase.from("product_cost_sheets").upsert(plan.sheets, { onConflict: "id" })),
  ];
  for (const write of writes) {
    if (!write) continue;
    const { error } = await write();
    if (error) return { ...none, error: describeError(error) };
  }
  return { products: plan.products.length, lines: plan.lines.length, sheets: plan.sheets.length };
}
