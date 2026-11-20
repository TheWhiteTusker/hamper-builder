/**
 * The Product Master workbook's costing: each product's rows on the Cost Lines
 * tab replace its saved lines, with the overhead % from its Product Master row.
 * A product with no Cost Lines rows keeps its costing. Planned on the dry run so
 * warnings show before Save; written after the products. Server-only, and a few
 * bulk requests however long the sheet is.
 */
import { isAdmin, requireUser, type createClient } from "@/lib/supabase/server";
import { body, cell, type Grid, type ImportResult } from "@/lib/import";
import { mrpLimit } from "@/lib/mrp";
import { planSheetCosting, readCostLines, readOverheads, type CostingPlan } from "@/lib/product-sheet-costing";
import { loadSettings } from "@/lib/settings";
import type { ProductCostLine } from "@/lib/types";
import { loadCostStages } from "../../cost-calculator/load";
import { repriceVarieties } from "../../cost-calculator/reprice";
import { CHUNK, inChunks, marginOf } from "./save";

type Supabase = Awaited<ReturnType<typeof createClient>>;
type Row = {
  code: string;
  deleted_at: string | null;
  product_cost_sheets: { id: string; stage_overheads: Record<string, number> | null; product_cost_lines: ProductCostLine[] }[];
};

/**
 * One plan per product whose costing the sheet changes; sets its cost_price and
 * margin. `wastage` holds the Rates page wastage % the sheet changes.
 */
export async function planCostings(supabase: Supabase, result: ImportResult, products: Grid = [], linesGrid?: Grid) {
  const errors: string[] = [];
  const wastage = new Map<string, number>();
  if (!body(linesGrid).length) return { plans: [], errors, wastage };
  const sheetLines = readCostLines(linesGrid, await loadCostStages(supabase), result.warnings, errors, wastage);
  if (wastage.size && !isAdmin((await requireUser()).profile.role)) {
    errors.push("Only an admin can change a rate's wastage, because it updates the products that use it.");
  }

  const overheads = new Map(body(products).map((row) => [cell(row, 0), readOverheads(row)]));
  const strays = [...sheetLines.keys()].filter((code) => !overheads.has(code));
  if (strays.length) result.warnings.push(`Cost Lines for codes not on the Product Master tab are ignored: ${strays.slice(0, 8).join(", ")}`);

  const costed = result.products.filter((p) => sheetLines.has(p.code));
  if (!costed.length || errors.length) return { plans: [], errors, wastage };

  // A live product wins over binned copies of its code, as in saveByCode.
  const rows = new Map<string, Row>();
  for (let i = 0; i < costed.length; i += 200) {
    const { data, error } = await supabase
      .from("products")
      .select("code, deleted_at, product_cost_sheets(id, stage_overheads, product_cost_lines(*))")
      .in("code", costed.slice(i, i + 200).map((p) => p.code))
      .returns<Row[]>();
    if (error) throw error;
    for (const r of data) if (!rows.has(r.code) || r.deleted_at === null) rows.set(r.code, r);
  }

  const { gst_rate } = await loadSettings(supabase);
  const plans = new Map<string, CostingPlan>();
  for (const p of costed) {
    const sheet = rows.get(p.code)?.product_cost_sheets[0];
    const existing = sheet && { sheetId: sheet.id, overheads: sheet.stage_overheads ?? {}, lines: sheet.product_cost_lines };
    const plan = planSheetCosting(p, sheetLines.get(p.code)!, overheads.get(p.code) ?? {}, existing, result.warnings);
    if (!plan) continue;
    p.cost_price = plan.totals.total_cost;
    p.target_margin = marginOf(p.default_sp, p.cost_price);
    const limit = mrpLimit(plan.lines, plan.overheads, plan.totals.total_cost, gst_rate);
    if (limit != null && p.default_sp > limit) result.warnings.push(`${p.code}: SP ₹${p.default_sp} is above its MRP limit of ₹${limit}.`);
    plans.set(p.code, plan);
  }
  return { plans: [...plans.values()], errors, wastage };
}

/**
 * Writes the sheet's wastage changes to the Rates page and reprices every
 * product using those rates. Run before the sheet's own products are saved,
 * so their typed prices land last.
 */
export async function saveSheetWastage(supabase: Supabase, wastage: Map<string, number>) {
  for (const [id, default_wastage_pct] of wastage) {
    const { error } = await supabase.from("cost_varieties").update({ default_wastage_pct }).eq("id", id);
    if (error) throw error;
  }
  const res = await repriceVarieties(supabase, [...wastage.keys()]);
  if (res.error) throw new Error(`Wastage saved, but repricing failed: ${res.error}`);
}

/** Writes the plans once their products are saved (new ones included). */
export async function saveSheetCosting(supabase: Supabase, plans: CostingPlan[], products: ImportResult["products"]) {
  if (!plans.length) return;
  const product = new Map(products.map((p) => [p.code, p]));
  const productId = new Map<string, string>();
  for (let i = 0; i < plans.length; i += 200) {
    const { data, error } = await supabase
      .from("products")
      .select("id, code")
      .in("code", plans.slice(i, i + 200).map((p) => p.code))
      .is("deleted_at", null);
    if (error) throw error;
    for (const r of data) productId.set(r.code, r.id);
  }

  const sheetValues = ({ code, totals: t, overheads }: CostingPlan) => {
    const p = product.get(code)!;
    return {
      product_id: productId.get(code),
      product_code: code,
      product_name: p.name,
      material_total: t.material_total,
      hardware_total: t.hardware_total,
      finishing_total: t.finishing_total,
      machine_total: t.machine_total,
      total_cost: t.total_cost,
      markup_pct: p.markup_pct ?? 0,
      calculated_sp: p.default_sp,
      stage_overheads: overheads,
    };
  };

  const existing = plans.filter((p) => p.sheetId);
  const sheetId = new Map(existing.map((p) => [p.code, p.sheetId!]));
  await inChunks(
    existing.map((p) => ({ id: p.sheetId, ...sheetValues(p) })),
    (batch) => supabase.from("product_cost_sheets").upsert(batch, { onConflict: "id" }),
  );
  const fresh = plans.filter((p) => !p.sheetId && productId.has(p.code));
  for (let i = 0; i < fresh.length; i += CHUNK) {
    const { data, error } = await supabase
      .from("product_cost_sheets")
      .insert(fresh.slice(i, i + CHUNK).map(sheetValues))
      .select("id, product_code");
    if (error) throw error;
    for (const s of data) sheetId.set(s.product_code, s.id);
  }

  // The sheet's lines replace the saved ones.
  const replaced = existing.map((p) => p.sheetId!);
  for (let i = 0; i < replaced.length; i += 200) {
    const { error } = await supabase.from("product_cost_lines").delete().in("sheet_id", replaced.slice(i, i + 200));
    if (error) throw error;
  }
  const lines = plans.flatMap((p) => (sheetId.has(p.code) ? p.lines.map((l) => ({ ...l, sheet_id: sheetId.get(p.code)! })) : []));
  await inChunks(lines, (batch) => supabase.from("product_cost_lines").insert(batch));
}
