"use server";

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { CostSaveResult } from "@/lib/cost-tree";
import type { CostCategory } from "@/lib/types";

// No revalidatePath in the master's actions: it would re-render the whole
// hierarchy for every edit. Each returns the rows it changed and the view
// patches just those (see applyCostChanges).

export async function saveCostCategory(formData: FormData): Promise<CostSaveResult> {
  const supabase = await createClient();
  const id = formData.get("id")?.toString();
  const stage_id = formData.get("stage_id")?.toString();
  const name = formData.get("name")?.toString().trim();
  const sort_order = Number(formData.get("sort_order")) || 0;

  if (!id && !stage_id) return { error: "Stage is required." };
  if (!name) return { error: "Category name is required." };

  const { data, error } = id
    ? await supabase.from("cost_categories").update({ name, sort_order }).eq("id", id).select().single<CostCategory>()
    : await supabase.from("cost_categories").insert({ stage_id, name, sort_order }).select().single<CostCategory>();
  if (error) return { error: describeError(error) };

  return { ok: true, changes: [{ table: "categories", upsert: [data] }] };
}

export async function renameCostCategory(id: string, name: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  const trimmed = name.trim();
  if (!trimmed) return { error: "Category name cannot be empty." };

  const { data: cat } = await supabase
    .from("cost_categories")
    .select("name, stage_id")
    .eq("id", id)
    .maybeSingle<{ name: string; stage_id: string }>();

  if (!cat) return { error: "Category not found." };
  if (cat.name === trimmed) return { ok: true };

  // Check unique in stage
  const { data: existing } = await supabase
    .from("cost_categories")
    .select("id")
    .eq("stage_id", cat.stage_id)
    .ilike("name", trimmed)
    .neq("id", id)
    .maybeSingle();

  if (existing) {
    return { error: `A category named "${trimmed}" already exists in this stage.` };
  }

  const { data: renamed, error } = await supabase
    .from("cost_categories")
    .update({ name: trimmed })
    .eq("id", id)
    .select()
    .single<CostCategory>();
  if (error) return { error: describeError(error) };

  // Update line snapshots for varieties in this category
  const { data: subs } = await supabase
    .from("cost_subcategories")
    .select("id")
    .eq("category_id", id);
  const subIds = (subs ?? []).map((s: { id: string }) => s.id);
  if (subIds.length > 0) {
    const { data: vars } = await supabase
      .from("cost_varieties")
      .select("id")
      .in("subcategory_id", subIds);
    const varIds = (vars ?? []).map((v: { id: string }) => v.id);
    if (varIds.length > 0) {
      await supabase
        .from("product_cost_lines")
        .update({ category_name: trimmed })
        .in("cost_variety_id", varIds);
    }
  }

  return {
    ok: true,
    message: `Category renamed to "${trimmed}".`,
    changes: [{ table: "categories", upsert: [renamed] }],
  };
}

export async function deleteCostCategory(id: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_categories").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  return { ok: true, changes: [{ table: "categories", remove: [id] }] };
}
