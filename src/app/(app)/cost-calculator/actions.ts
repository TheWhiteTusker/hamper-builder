"use server";

import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { productHref } from "../products/href";
import { createClient } from "@/lib/supabase/server";
import { calculateCostSheetTotals } from "@/lib/costing.ts";
import { roundUpToNext10 } from "@/lib/numbers.ts";
import { describeError } from "@/lib/forms";
import { getNextSerialForCategory } from "@/lib/product-code";
import type { ProductCostLine } from "@/lib/types";
import { prepareLines, saveColorVariants, saveRow } from "./save-sheet";

export type SaveCostSheetPayload = {
  sheetId?: string | null;
  productId?: string | null;
  productCode: string;
  productName: string;
  categoryId?: string | null;
  source?: string | null;
  colors?: string[];
  isActive?: boolean;
  markupPct: number;
  /** Overhead % per stage code, added to that stage's subtotal. */
  stageOverheads?: Record<string, number>;
  sellingPrice?: number;
  notes?: string | null;
  lines: ProductCostLine[];
};

export async function saveCostSheetAndProduct(payload: SaveCostSheetPayload) {
  try {
    const supabase = await createClient();
    const {
      data: { user },
    } = await supabase.auth.getUser();

    if (!user) {
      return { error: "You must be signed in to save costing." };
    }

    const code = payload.productCode.trim();
    const name = payload.productName.trim();

    if (!code) return { error: "Product code is required." };
    if (!name) return { error: "Product name is required." };

    const preparedLines = prepareLines(payload.lines);
    // Only real, non-zero percentages are kept.
    const stageOverheads = Object.fromEntries(
      Object.entries(payload.stageOverheads ?? {})
        .map(([stage, pct]) => [stage.toLowerCase(), Number(pct)] as const)
        .filter(([, pct]) => Number.isFinite(pct) && pct !== 0),
    );
    const totals = calculateCostSheetTotals(preparedLines, payload.markupPct, stageOverheads);
    const rawSellingPrice =
      payload.sellingPrice != null && payload.sellingPrice > 0
        ? payload.sellingPrice
        : totals.calculated_sp;
    const finalSellingPrice = roundUpToNext10(rawSellingPrice);

    const targetMargin =
      finalSellingPrice > 0 ? (finalSellingPrice - totals.total_cost) / finalSellingPrice : 0;

    // 1. Save the product. Several colours means one product per colour
    // (LC/0001/WL, LC/0001/BL): this one takes the first colour, the rest
    // become sibling products in step 4.
    const colors = payload.colors ?? [];
    const productValues = {
      code,
      name,
      category_id: payload.categoryId || null,
      source: payload.source || null,
      cost_price: totals.total_cost,
      markup_pct: totals.markup_pct,
      target_margin: targetMargin,
      default_sp: finalSellingPrice,
      colors: colors.slice(0, 1),
      is_active: payload.isActive !== false,
    };
    const product = await saveRow(supabase, "products", productValues, payload.productId, ["code", code]);
    if (product.error !== undefined) return { error: `Error saving product: ${product.error}` };
    const productId = product.id;

    // 2. Save its cost sheet
    const sheetValues = {
      product_name: name,
      material_total: totals.material_total,
      hardware_total: totals.hardware_total,
      finishing_total: totals.finishing_total,
      machine_total: totals.machine_total,
      total_cost: totals.total_cost,
      markup_pct: totals.markup_pct,
      calculated_sp: finalSellingPrice,
      stage_overheads: stageOverheads,
      notes: payload.notes || null,
      created_by: user.id,
    };
    const sheet = await saveRow(
      supabase,
      "product_cost_sheets",
      {
        ...sheetValues,
        product_id: productId,
        product_code: code,
      },
      payload.sheetId,
      ["product_id", productId],
    );
    if (sheet.error !== undefined) return { error: `Error saving cost sheet: ${sheet.error}` };
    const sheetId = sheet.id;

    // 3. Replace lines for this sheet
    await supabase.from("product_cost_lines").delete().eq("sheet_id", sheetId);
    if (preparedLines.length > 0) {
      const { error: lErr } = await supabase
        .from("product_cost_lines")
        .insert(preparedLines.map((line) => ({ ...line, sheet_id: sheetId })));
      if (lErr) return { error: `Error saving cost lines: ${describeError(lErr)}` };
    }

    // 4. One sibling product per extra colour, including their cost sheets
    const saved = await saveColorVariants(
      supabase,
      code,
      productId,
      colors,
      productValues,
      sheetValues,
      preparedLines,
    );
    if (saved.error !== undefined) return { error: saved.error };

    // No revalidatePath here: it makes Next re-render the whole calculator into
    // this response, which the client throws away to open the product page.
    // The client calls costingSaved() once photos are up.
    return {
      ok: true,
      sheetId,
      productId,
      variants: saved.variants,
      productCode: code,
      totalCost: totals.total_cost,
      sellingPrice: finalSellingPrice,
    };
  } catch (err: unknown) {
    return { error: err instanceof Error ? err.message : String(err) };
  }
}

/**
 * Refreshes the pages a costing save changes, then opens `openCode`'s product
 * page in this same request. Without `openCode`, re-renders the current page.
 * Kept apart from the save so the Worker renders one page per save, not two.
 */
export async function costingSaved(openCode?: string) {
  revalidatePath("/cost-calculator", "layout");
  revalidatePath("/products", "layout");
  revalidatePath("/hampers");
  if (openCode) redirect(productHref(openCode));
}

export async function getNextSerialAction(
  categoryCode: string,
): Promise<{ serial: string; error?: string }> {
  try {
    const supabase = await createClient();
    const { data: products, error } = await supabase.from("products").select("code");
    if (error) return { serial: "0001", error: describeError(error) };
    const codes = (products ?? []).map((p: { code: string }) => p.code);
    return { serial: getNextSerialForCategory(categoryCode, codes) };
  } catch (err: unknown) {
    return { serial: "0001", error: describeError(err) };
  }
}
