"use server";

import { revalidatePath } from "next/cache";
import type { SupabaseClient } from "@supabase/supabase-js";
import { isAdmin, requireUser } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { CostRows } from "@/lib/cost-tree";
import type { Grid } from "@/lib/import-cells";
import { planRateImport } from "@/lib/rate-import";
import type { ImportReport } from "../../../admin/import/actions";
import { loadCostRows } from "../../load";
import { repriceVarieties } from "../../reprice";

async function adminClient() {
  const { supabase, profile } = await requireUser();
  if (!isAdmin(profile.role)) throw new Error("Only an admin can change the master from Excel.");
  return supabase;
}

/** The active hierarchy, for the downloadable sheet. */
export async function rateRowsForSheet(): Promise<CostRows> {
  return loadCostRows(await adminClient());
}

/**
 * Every row, inactive too: names are unique per parent either way.
 * ponytail: one request per table, so capped at Supabase's 1000 rows each; past that a
 * missed variety reads as new and its insert fails on the unique name (safe, not silent). Page with .range() then.
 */
async function loadAllCostRows(supabase: SupabaseClient): Promise<CostRows> {
  const all = (table: string) => supabase.from(table).select("*").order("sort_order");
  const res = await Promise.all(["cost_stages", "cost_categories", "cost_subcategories", "cost_varieties"].map(all));
  const failed = res.find((r) => r.error);
  if (failed) throw failed.error;
  const [stages, categories, subcategories, varieties] = res.map((r) => r.data ?? []);
  return { stages, categories, subcategories, varieties } as CostRows;
}

const must = <T,>({ data, error }: { data: T | null; error: unknown }) => {
  if (error) throw error;
  return data as T;
};

/**
 * Dry run unless `apply`. A handful of bulk requests however long the sheet
 * is: a Worker invocation caps its outgoing requests.
 */
export async function importRateSheet(grid: Grid, apply: boolean, onlyNew: boolean): Promise<ImportReport> {
  try {
    const supabase = await adminClient();
    const plan = planRateImport(grid, await loadAllCostRows(supabase), onlyNew);
    if (!apply) return { ok: true, applied: false, counts: plan.counts, warnings: plan.warnings };

    const { ids } = plan;
    for (const [table, list] of [
      ["cost_categories", plan.reactivate.categories],
      ["cost_subcategories", plan.reactivate.subcategories],
    ] as const) {
      if (list.length) must(await supabase.from(table).update({ is_active: true }).in("id", list));
    }

    // New categories, then subcategories under them, then varieties; each insert's
    // ids feed the next. Keys are "stageId|category|subcategory" in lower case.
    const catKeyById = new Map<string, string>();
    if (plan.newCategories.size) {
      const rows = must(await supabase.from("cost_categories").insert([...plan.newCategories.values()]).select("id, stage_id, name"));
      for (const c of rows as { id: string; stage_id: string; name: string }[]) {
        const key = `${c.stage_id}|${c.name.trim().toLowerCase()}`;
        ids.set(key, c.id);
      }
    }
    for (const [key, id] of ids) catKeyById.set(id, key);

    if (plan.newSubcategories.size) {
      const insert = [...plan.newSubcategories.values()].map(({ catKey, name, sort_order }) => ({
        category_id: ids.get(catKey)!,
        name,
        sort_order,
      }));
      const rows = must(await supabase.from("cost_subcategories").insert(insert).select("id, category_id, name"));
      for (const s of rows as { id: string; category_id: string; name: string }[]) {
        ids.set(`${catKeyById.get(s.category_id)}|${s.name.trim().toLowerCase()}`, s.id);
      }
    }

    if (plan.newVarieties.length) {
      const insert = plan.newVarieties.map(({ subKey, name, default_rate, unit, default_wastage_pct, notes, sort_order }) => ({
        subcategory_id: ids.get(subKey)!, name, default_rate, unit, default_wastage_pct, notes, sort_order,
      }));
      must(await supabase.from("cost_varieties").insert(insert));
    }
    if (plan.updates.length) {
      must(await supabase.from("cost_varieties").upsert(plan.updates, { onConflict: "id" }));
    }

    const repriced = await repriceVarieties(supabase, plan.reprice);
    revalidatePath("/cost-calculator", "layout");
    if (plan.reprice.length) {
      revalidatePath("/products");
      revalidatePath("/products/[...code]", "page");
    }
    if (repriced.error) {
      return { ok: false, applied: true, counts: plan.counts, error: `Master saved, but repricing failed: ${repriced.error}` };
    }
    return {
      ok: true,
      applied: true,
      counts: plan.counts,
      warnings: plan.warnings,
      added: plan.newVarieties.map((v) => ({ label: v.label })),
      message: repriced.products
        ? `Master saved. ${repriced.products} product${repriced.products === 1 ? "" : "s"} repriced; quotations and invoices keep their prices.`
        : "Master saved.",
    };
  } catch (error) {
    return { ok: false, applied: false, error: describeError(error) };
  }
}
