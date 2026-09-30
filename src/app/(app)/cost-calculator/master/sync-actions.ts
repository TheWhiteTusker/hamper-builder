"use server";

import { revalidatePath } from "next/cache";
import { isAdmin, requireUser } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { repriceVarieties } from "../reprice";

type StoredLine = {
  sheet_id: string;
  cost_variety_id: string;
  rate: number;
  unit: string;
  variety_name: string | null;
  subcategory_name: string | null;
};
type MasterVariety = {
  id: string;
  name: string;
  default_rate: number;
  unit: string;
  cost_subcategories: { name: string } | null;
};

/**
 * Brings every saved costing up to the master: lines whose rate, unit or
 * names differ from their variety are re-rated, and their products repriced
 * (same rules as editing a rate). Quotations and invoices are not touched.
 */
export async function syncAllRates(): Promise<{ error?: string; message?: string }> {
  const { supabase, profile } = await requireUser();
  if (!isAdmin(profile.role)) return { error: "Only an admin can sync products with the master rates." };

  const [{ data: lines, error }, { data: varieties, error: vError }] = await Promise.all([
    supabase
      .from("product_cost_lines")
      .select("sheet_id, cost_variety_id, rate, unit, variety_name, subcategory_name")
      .not("cost_variety_id", "is", null)
      .returns<StoredLine[]>(),
    supabase
      .from("cost_varieties")
      .select("id, name, default_rate, unit, cost_subcategories(name)")
      .returns<MasterVariety[]>(),
  ]);
  if (error || vError) return { error: describeError(error ?? vError) };

  const master = new Map((varieties ?? []).map((v) => [v.id, v]));
  const stale = (lines ?? []).filter((l) => {
    const v = master.get(l.cost_variety_id);
    return (
      !!v &&
      (Number(l.rate) !== Number(v.default_rate) ||
        l.unit !== v.unit ||
        l.variety_name !== v.name ||
        l.subcategory_name !== (v.cost_subcategories?.name ?? l.subcategory_name))
    );
  });
  if (!stale.length) return { message: "Every product already uses the current master rates and names." };

  const res = await repriceVarieties(supabase, [...new Set(stale.map((l) => l.cost_variety_id))]);
  if (res.error) return { error: `Sync failed: ${res.error}` };

  revalidatePath("/cost-calculator", "layout");
  revalidatePath("/products");
  revalidatePath("/products/[...code]", "page");
  const sheets = new Set(stale.map((l) => l.sheet_id)).size;
  return {
    message: `Updated ${stale.length} line${stale.length === 1 ? "" : "s"} on ${sheets} product costing${sheets === 1 ? "" : "s"}. Quotations and invoices keep their prices.`,
  };
}
