/** Bulk writes for the importers. Server-only: imported by the actions, never by a page. */
import { createClient } from "@/lib/supabase/server";
import type { ImportResult } from "@/lib/import";
import { colorCode, parseProductCode } from "@/lib/product-code";
import { loadSettings } from "@/lib/settings";

export const CHUNK = 500;

export async function inChunks<T>(rows: T[], fn: (batch: T[]) => PromiseLike<{ error: unknown }>) {
  for (let i = 0; i < rows.length; i += CHUNK) {
    const { error } = await fn(rows.slice(i, i + CHUNK));
    if (error) throw error;
  }
}

type Supabase = Awaited<ReturnType<typeof createClient>>;

/**
 * Insert-or-update on `code`. Not upsert({ onConflict: "code" }): since the Bin,
 * code is unique only among live rows (a partial index), which ON CONFLICT
 * cannot target. A code matching a binned row brings it back, as the product
 * form does. Batches of 200 keep the `in` list inside the URL limit.
 * Returns the codes that were new.
 */
export async function saveByCode<T extends { code: string }>(supabase: Supabase, table: "products" | "hampers", rows: T[]) {
  const added: string[] = [];
  for (let i = 0; i < rows.length; i += 200) {
    const batch = rows.slice(i, i + 200);
    const { data, error } = await supabase
      .from(table)
      .select("id, code, deleted_at")
      .in("code", batch.map((r) => r.code));
    if (error) throw error;

    // A live row wins over binned copies of the same code.
    const idByCode = new Map<string, string>();
    for (const r of data as { id: string; code: string; deleted_at: string | null }[]) {
      if (!idByCode.has(r.code) || r.deleted_at === null) idByCode.set(r.code, r.id);
    }

    const updates = batch
      .filter((r) => idByCode.has(r.code))
      .map((r) => ({ ...r, id: idByCode.get(r.code)!, deleted_at: null }));
    const inserts = batch.filter((r) => !idByCode.has(r.code));
    if (updates.length) {
      const { error } = await supabase.from(table).upsert(updates, { onConflict: "id" });
      if (error) throw error;
    }
    if (inserts.length) {
      const { error } = await supabase.from(table).insert(inserts);
      if (error) throw error;
      added.push(...inserts.map((r) => r.code));
    }
  }
  return added;
}

/** The stored margin a price gives, as the cost calculator works it out. */
export const marginOf = (sp: number, cost: number) => (sp > 0 ? Math.round(((sp - cost) / sp) * 10000) / 10000 : 0);

/**
 * Rows from the Product Master Excel sheet. Column F is Markup % there (buildImport
 * reads it into target_margin), and its SP may be the formula or typed over, so the
 * stored margin is worked out from the price given, as the cost calculator does.
 * "add" drops codes that already exist (Bin included). A new product takes its
 * colour from its code (LC/0001/NT is Natural); returns an error when that isn't
 * a colour in Settings, as every product must have one.
 */
export async function prepareSheetProducts(supabase: Supabase, result: ImportResult, sheet: "add" | "edit") {
  for (const p of result.products) {
    p.markup_pct = Math.round(p.target_margin * 10000) / 100;
    p.target_margin = marginOf(p.default_sp, p.cost_price);
  }
  if (!result.products.length) return;

  const codes = result.products.map((p) => p.code);
  const existing = new Set<string>();
  for (let i = 0; i < codes.length; i += 200) {
    const { data, error } = await supabase.from("products").select("code").in("code", codes.slice(i, i + 200));
    if (error) throw error;
    data.forEach((p) => existing.add(p.code));
  }
  const list = (xs: string[]) => xs.slice(0, 8).join(", ") + (xs.length > 8 ? "…" : "");

  if (sheet === "add" && existing.size) {
    result.products = result.products.filter((p) => !existing.has(p.code));
    result.counts.products = result.products.length;
    result.warnings.push(`${existing.size} code(s) already exist and will be skipped. Use Bulk edit to change them: ${list([...existing])}`);
  }

  const { product_colors } = await loadSettings(supabase);
  const colorBy = new Map(product_colors.map((c) => [colorCode(c), c]));
  const uncoloured: string[] = [];
  for (const p of result.products) {
    if (existing.has(p.code)) continue;
    const { isValid, colorCode: suffix } = parseProductCode(p.code);
    const color = isValid ? colorBy.get(suffix) : undefined;
    if (color) p.colors = [color];
    else uncoloured.push(p.code);
  }
  if (!uncoloured.length) return;
  const known = [...colorBy].map(([code, name]) => `${code} ${name}`).join(", ");
  return `New product codes must look like AC/0006/NT, ending in a colour code from Settings (${known}). Check: ${list(uncoloured)}`;
}
