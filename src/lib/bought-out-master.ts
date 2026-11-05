import type { SupabaseClient } from "@supabase/supabase-js";
import type { ProductCostLine } from "./types.ts";

type Row = { id: string; name: string; is_active: boolean; default_rate?: number; unit?: string };

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();
const fail = (error: { message: string } | null) => {
  if (error) throw new Error(error.message);
};

/** The row under `parentId` named `name` (any case): found, re-activated or created. */
async function findOrCreate(
  supabase: SupabaseClient,
  table: string,
  parentCol: string,
  parentId: string,
  name: string,
  values: Record<string, unknown> = {},
): Promise<Row> {
  const { data, error } = await supabase.from(table).select("*").eq(parentCol, parentId);
  fail(error);
  const hit = (data as Row[] | null)?.find((r) => same(r.name, name));
  if (hit) {
    // The master only lists active rows; a typed name brings a hidden one back.
    if (!hit.is_active) fail((await supabase.from(table).update({ is_active: true }).eq("id", hit.id)).error);
    return hit;
  }
  const { data: row, error: insertError } = await supabase
    .from(table)
    .insert({ [parentCol]: parentId, name: name.trim(), ...values })
    .select("*")
    .single();
  fail(insertError);
  return row as Row;
}

/**
 * Outsourced products type their bought-out items instead of picking them.
 * Each typed Category -> Subcategory -> Variety goes into the Rates & Hierarchy
 * Master (created when new, its rate and unit updated when changed) and the
 * line is linked to it. Returns the linked lines and the varieties whose rate
 * changed, so the other products costed with them can be repriced.
 */
export async function linkBoughtOutLines(
  supabase: SupabaseClient,
  lines: ProductCostLine[],
): Promise<{ lines: ProductCostLine[]; repriced: string[]; error?: undefined } | { error: string }> {
  const bought = lines.filter((l) => l.stage_code === "bought_out");
  const incomplete = bought.find(
    (l) => !l.category_name?.trim() || !l.subcategory_name?.trim() || !l.variety_name?.trim(),
  );
  if (incomplete) {
    const label = incomplete.item_name || incomplete.category_name || "a bought-out line";
    return { error: `Fill Category, Subcategory and Variety for "${label}" to save it to the Rates & Hierarchy Master.` };
  }
  if (!bought.length) return { lines, repriced: [] };

  try {
    const { data: stage, error } = await supabase.from("cost_stages").select("id").eq("code", "bought_out").maybeSingle();
    fail(error);
    if (!stage) return { error: "The Bought Out Items stage is missing from the Rates & Hierarchy Master." };

    const repriced = new Set<string>();
    const linked: ProductCostLine[] = [];
    // ponytail: a few requests per line, fine for a handful of bought-out items.
    for (const l of lines) {
      if (l.stage_code !== "bought_out") {
        linked.push(l);
        continue;
      }
      const rate = Number(l.rate) || 0;
      const cat = await findOrCreate(supabase, "cost_categories", "stage_id", stage.id, l.category_name);
      const sub = await findOrCreate(supabase, "cost_subcategories", "category_id", cat.id, l.subcategory_name!);
      const v = await findOrCreate(supabase, "cost_varieties", "subcategory_id", sub.id, l.variety_name!, {
        default_rate: rate,
        unit: l.unit,
        default_wastage_pct: 0,
      });
      if (Number(v.default_rate) !== rate || v.unit !== l.unit) {
        fail((await supabase.from("cost_varieties").update({ default_rate: rate, unit: l.unit }).eq("id", v.id)).error);
        repriced.add(v.id);
      }
      linked.push({
        ...l,
        category_name: cat.name,
        subcategory_name: sub.name,
        variety_name: v.name,
        item_name: `${sub.name} ${v.name}`,
        cost_variety_id: v.id,
      });
    }
    return { lines: linked, repriced: [...repriced] };
  } catch (err) {
    return { error: `Saving bought-out items to the master failed: ${err instanceof Error ? err.message : String(err)}` };
  }
}
