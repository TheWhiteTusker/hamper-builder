/** Bulk writes for the importers. Server-only: imported by the actions, never by a page. */
import { createClient } from "@/lib/supabase/server";
import type { ImportResult } from "@/lib/import";

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

/**
 * Rows from the Product Master Excel sheet. Column F is Markup % there (buildImport
 * reads it into target_margin), and its SP may be the formula or typed over, so the
 * stored margin is worked out from the price given, as the cost calculator does.
 * "add" drops codes that already exist (Bin included).
 */
export async function prepareSheetProducts(supabase: Supabase, result: ImportResult, sheet: "add" | "edit") {
  for (const p of result.products) {
    p.markup_pct = Math.round(p.target_margin * 10000) / 100;
    p.target_margin = p.default_sp > 0 ? Math.round(((p.default_sp - p.cost_price) / p.default_sp) * 10000) / 10000 : 0;
  }
  if (sheet !== "add" || !result.products.length) return;

  const codes = result.products.map((p) => p.code);
  const existing = new Set<string>();
  for (let i = 0; i < codes.length; i += 200) {
    const { data, error } = await supabase.from("products").select("code").in("code", codes.slice(i, i + 200));
    if (error) throw error;
    data.forEach((p) => existing.add(p.code));
  }
  if (!existing.size) return;

  result.products = result.products.filter((p) => !existing.has(p.code));
  result.counts.products = result.products.length;
  const list = [...existing].slice(0, 8).join(", ") + (existing.size > 8 ? "…" : "");
  result.warnings.push(`${existing.size} code(s) already exist and will be skipped. Use “Edit & add products” to change them: ${list}`);
}
