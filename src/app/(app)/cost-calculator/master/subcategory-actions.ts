"use server";

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { CostSaveResult } from "@/lib/cost-tree";
import type { CostSubcategory } from "@/lib/types";

// Return the changed rows instead of revalidatePath; see category-actions.ts.

export async function saveCostSubcategory(formData: FormData): Promise<CostSaveResult> {
  const supabase = await createClient();
  const id = formData.get("id")?.toString();
  const category_id = formData.get("category_id")?.toString();
  const name = formData.get("name")?.toString().trim();
  const sort_order = Number(formData.get("sort_order")) || 0;

  if (!category_id) return { error: "Category is required." };
  if (!name) return { error: "Subcategory name is required (e.g. Birch, Acacia)." };

  const { data, error } = id
    ? await supabase.from("cost_subcategories").update({ name, sort_order }).eq("id", id).select().single<CostSubcategory>()
    : await supabase.from("cost_subcategories").insert({ category_id, name, sort_order }).select().single<CostSubcategory>();
  if (error) return { error: describeError(error) };

  return { ok: true, changes: [{ table: "subcategories", upsert: [data] }] };
}

export async function renameCostSubcategory(id: string, name: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  const trimmed = name.trim();
  if (!trimmed) return { error: "Subcategory name cannot be empty." };

  const { data: sub } = await supabase
    .from("cost_subcategories")
    .select("name, category_id")
    .eq("id", id)
    .maybeSingle<{ name: string; category_id: string }>();

  if (!sub) return { error: "Subcategory not found." };
  if (sub.name === trimmed) return { ok: true };

  // Check unique in category
  const { data: existing } = await supabase
    .from("cost_subcategories")
    .select("id")
    .eq("category_id", sub.category_id)
    .ilike("name", trimmed)
    .neq("id", id)
    .maybeSingle();

  if (existing) {
    return { error: `A subcategory named "${trimmed}" already exists in this category.` };
  }

  const { data: renamed, error } = await supabase
    .from("cost_subcategories")
    .update({ name: trimmed })
    .eq("id", id)
    .select()
    .single<CostSubcategory>();
  if (error) return { error: describeError(error) };

  // Update line snapshots for varieties in this subcategory
  const { data: vars } = await supabase
    .from("cost_varieties")
    .select("id, name")
    .eq("subcategory_id", id)
    .returns<{ id: string; name: string }[]>();

  if (vars && vars.length > 0) {
    for (const v of vars) {
      await supabase
        .from("product_cost_lines")
        .update({
          subcategory_name: trimmed,
          item_name: `${trimmed} ${v.name}`,
        })
        .eq("cost_variety_id", v.id);
    }
  }

  return {
    ok: true,
    message: `Subcategory renamed to "${trimmed}".`,
    changes: [{ table: "subcategories", upsert: [renamed] }],
  };
}

export async function deleteCostSubcategory(id: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_subcategories").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  return { ok: true, changes: [{ table: "subcategories", remove: [id] }] };
}
