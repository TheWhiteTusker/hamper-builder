"use server";

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { CostSaveResult } from "@/lib/cost-tree";
import type { CostSubcategory, CostVariety } from "@/lib/types";

// Return the changed rows instead of revalidatePath; see category-actions.ts.

type Supabase = Awaited<ReturnType<typeof createClient>>;

/** One past the highest sort_order in a category, so the subcategory lands last. */
async function nextSortOrder(supabase: Supabase, categoryId: string) {
  const { data } = await supabase
    .from("cost_subcategories")
    .select("sort_order")
    .eq("category_id", categoryId)
    .order("sort_order", { ascending: false })
    .limit(1)
    .returns<{ sort_order: number }[]>();
  return (data?.[0]?.sort_order ?? 0) + 1;
}

/** Whether the category already has a subcategory by this name. */
async function nameTaken(supabase: Supabase, categoryId: string, name: string) {
  const { data } = await supabase
    .from("cost_subcategories")
    .select("id")
    .eq("category_id", categoryId)
    .ilike("name", name)
    .maybeSingle();
  return !!data;
}

export async function moveCostSubcategory(subcategoryId: string, targetCategoryId: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  if (!subcategoryId || !targetCategoryId) {
    return { error: "Subcategory and destination category are required." };
  }

  const { data: sub } = await supabase
    .from("cost_subcategories")
    .select("id, name, category_id")
    .eq("id", subcategoryId)
    .maybeSingle<{ id: string; name: string; category_id: string }>();

  if (!sub) return { error: "Subcategory not found." };
  if (sub.category_id === targetCategoryId) {
    return { error: "The subcategory is already in that category." };
  }

  const { data: targetCat } = await supabase
    .from("cost_categories")
    .select("id, name, stage_id, cost_stages(code)")
    .eq("id", targetCategoryId)
    .maybeSingle<{ id: string; name: string; stage_id: string; cost_stages: { code: string } | null }>();

  if (!targetCat) return { error: "Destination category not found." };

  if (await nameTaken(supabase, targetCategoryId, sub.name)) {
    return {
      error: `A subcategory named "${sub.name}" already exists in "${targetCat.name}". Please rename it first or choose another category.`,
    };
  }

  const { data: moved, error: moveErr } = await supabase
    .from("cost_subcategories")
    .update({ category_id: targetCategoryId, sort_order: await nextSortOrder(supabase, targetCategoryId) })
    .eq("id", subcategoryId)
    .select()
    .single<CostSubcategory>();

  if (moveErr) return { error: describeError(moveErr) };

  // Update line snapshots
  const stageCode = targetCat.cost_stages?.code;
  const { data: vars } = await supabase
    .from("cost_varieties")
    .select("id")
    .eq("subcategory_id", subcategoryId)
    .returns<{ id: string }[]>();

  const varIds = (vars ?? []).map((v) => v.id);
  if (varIds.length > 0) {
    await supabase
      .from("product_cost_lines")
      .update({
        category_name: targetCat.name,
        ...(stageCode ? { stage_code: stageCode } : {}),
      })
      .in("cost_variety_id", varIds);
  }

  return {
    ok: true,
    message: `Subcategory "${sub.name}" moved to "${targetCat.name}".`,
    changes: [{ table: "subcategories", upsert: [moved] }],
  };
}

export async function duplicateCostSubcategory(
  subcategoryId: string,
  targetCategoryId: string,
  newName?: string,
): Promise<CostSaveResult> {
  const supabase = await createClient();
  if (!subcategoryId || !targetCategoryId) {
    return { error: "Subcategory and destination category are required." };
  }

  const [{ data: sub }, { data: varieties }] = await Promise.all([
    supabase
      .from("cost_subcategories")
      .select("id, name, category_id")
      .eq("id", subcategoryId)
      .maybeSingle<{ id: string; name: string; category_id: string }>(),
    supabase
      .from("cost_varieties")
      .select("*")
      .eq("subcategory_id", subcategoryId)
      .returns<CostVariety[]>(),
  ]);

  if (!sub) return { error: "Subcategory not found." };

  const { data: targetCat } = await supabase
    .from("cost_categories")
    .select("id, name")
    .eq("id", targetCategoryId)
    .maybeSingle<{ id: string; name: string }>();

  if (!targetCat) return { error: "Destination category not found." };

  const trimmedName = newName?.trim() || (sub.category_id === targetCategoryId ? `${sub.name} (Copy)` : sub.name);

  if (await nameTaken(supabase, targetCategoryId, trimmedName)) {
    return {
      error: `A subcategory named "${trimmedName}" already exists in "${targetCat.name}". Please pick a different name.`,
    };
  }

  // 1. Insert new subcategory
  const { data: newSub, error: subErr } = await supabase
    .from("cost_subcategories")
    .insert({
      category_id: targetCategoryId,
      name: trimmedName,
      sort_order: await nextSortOrder(supabase, targetCategoryId),
      is_active: true,
    })
    .select()
    .single<CostSubcategory>();

  if (subErr || !newSub) return { error: describeError(subErr ?? "Failed to create subcategory") };

  // 2. Insert duplicated varieties
  const vars = varieties ?? [];
  let newVars: CostVariety[] = [];
  if (vars.length > 0) {
    const varietyPayloads = vars.map((v) => ({
      subcategory_id: newSub.id,
      name: v.name,
      default_rate: v.default_rate,
      unit: v.unit,
      default_wastage_pct: v.default_wastage_pct,
      notes: v.notes,
      sort_order: v.sort_order,
      is_active: v.is_active,
    }));

    const { data, error: varErr } = await supabase
      .from("cost_varieties")
      .insert(varietyPayloads)
      .select()
      .returns<CostVariety[]>();
    if (varErr) {
      await supabase.from("cost_subcategories").delete().eq("id", newSub.id);
      return { error: describeError(varErr) };
    }
    // Inactive source varieties are copied too, but the master only shows active ones.
    newVars = (data ?? []).filter((v) => v.is_active);
  }

  return {
    ok: true,
    message: `Subcategory "${trimmedName}" created in "${targetCat.name}" with ${vars.length} varieties.`,
    changes: [
      { table: "subcategories", upsert: [newSub] },
      { table: "varieties", upsert: newVars },
    ],
  };
}
