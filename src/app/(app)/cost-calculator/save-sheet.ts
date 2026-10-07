import type { SupabaseClient } from "@supabase/supabase-js";
import { calculateLineCost } from "@/lib/costing.ts";
import { describeError } from "@/lib/forms";
import { codesForColors } from "@/lib/product-code";
import type { ProductCostLine } from "@/lib/types";

/** Cost lines as stored: recomputed totals, coerced numbers, 1-based order. */
export function prepareLines(lines: ProductCostLine[]): ProductCostLine[] {
  return lines.map((l, index) => {
    const calc = calculateLineCost(l);
    const subName = l.subcategory_name || "";
    const varName = l.variety_name || "";
    const itemName =
      l.item_name || (subName && varName ? `${subName} ${varName}` : subName || varName);

    return {
      stage_code: l.stage_code,
      category_name: l.category_name,
      subcategory_name: subName || null,
      variety_name: varName || null,
      item_name: itemName,
      cost_item_id: l.cost_item_id || null,
      cost_variety_id: l.cost_variety_id || null,
      length: l.length != null ? Number(l.length) : null,
      breadth: l.breadth != null ? Number(l.breadth) : null,
      dimension_unit: l.dimension_unit || "inch",
      unit: l.unit || "sq ft",
      rate: Number(l.rate) || 0,
      duration_minutes: l.duration_minutes != null ? Number(l.duration_minutes) : null,
      qty: l.qty != null ? Number(l.qty) : 1,
      wastage_pct: Number(l.wastage_pct) || 0,
      calculated_area: calc.calculated_area,
      line_total: calc.line_total,
      sort_order: index + 1,
    };
  });
}

/**
 * Update the row with `id`; without one, update the row whose `match` column
 * equals the value, or insert. Returns the row's id, or the error.
 */
export async function saveRow(
  supabase: SupabaseClient,
  table: string,
  values: Record<string, unknown>,
  id: string | null | undefined,
  [column, value]: [string, string],
): Promise<{ id: string; error?: undefined } | { id?: undefined; error: string }> {
  // When saving a product, always clear deleted_at so soft-deleted records in the bin are restored
  const rowValues = table === "products" ? { ...values, deleted_at: null } : values;

  if (!id) {
    const { data } = await supabase.from(table).select("id").eq(column, value).maybeSingle();
    id = data?.id as string | undefined;
  }
  if (id) {
    const { error } = await supabase.from(table).update(rowValues).eq("id", id);
    return error ? { error: describeError(error) } : { id };
  }
  const { data, error } = await supabase.from(table).insert(rowValues).select("id").single();
  return error || !data ? { error: describeError(error) } : { id: data.id as string };
}

/**
 * One sibling product per newly ticked colour, created as a copy of this one
 * (product, cost sheet and lines). Colours that already exist are left alone:
 * each colour is edited on its own. A binned sibling is restored as a copy.
 */
export async function saveColorVariants(
  supabase: SupabaseClient,
  code: string,
  productId: string,
  colors: string[],
  productValues: Record<string, unknown>,
  sheetValues?: Record<string, unknown>,
  lines?: Partial<ProductCostLine>[],
) {
  const variants: { color: string; code: string; id: string }[] = [];
  if (colors[0]) variants.push({ color: colors[0], code, id: productId });
  const extra = codesForColors(code, colors.slice(1)).filter((v) => v.code !== code);
  const { data: live } = await supabase
    .from("products")
    .select("id, code")
    .in("code", extra.map((v) => v.code))
    .is("deleted_at", null);
  const liveIds = new Map((live ?? []).map((p) => [p.code as string, p.id as string]));

  for (const { color, code: variantCode } of extra) {
    const existingId = liveIds.get(variantCode);
    if (existingId) {
      variants.push({ color, code: variantCode, id: existingId });
      continue;
    }
    const values = { ...productValues, code: variantCode, colors: [color], deleted_at: null };
    const saved = await saveRow(supabase, "products", values, null, ["code", variantCode]);
    if (saved.error !== undefined) return { error: `Error saving ${variantCode}: ${saved.error}` };
    const variantId = saved.id;
    variants.push({ color, code: variantCode, id: variantId });

    if (sheetValues) {
      const sheet = await saveRow(
        supabase,
        "product_cost_sheets",
        {
          ...sheetValues,
          product_id: variantId,
          product_code: variantCode,
        },
        null,
        ["product_id", variantId],
      );
      if (sheet.id && lines && lines.length > 0) {
        await supabase.from("product_cost_lines").delete().eq("sheet_id", sheet.id);
        await supabase
          .from("product_cost_lines")
          .insert(lines.map((l) => ({ ...l, sheet_id: sheet.id })));
      }
    }
  }
  return { variants };
}
