"use server";

import { revalidatePath } from "next/cache";
import { createClient, isAdmin, requireUser } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { ProductColor } from "@/lib/product-code";
import type { CostVariety } from "@/lib/types";
import { repriceVariety } from "../reprice";

// Every page under /cost-calculator reads the hierarchy, so refresh them all.
const refresh = () => revalidatePath("/cost-calculator", "layout");

export async function saveCostCategory(formData: FormData) {
  const supabase = await createClient();
  const id = formData.get("id")?.toString();
  const stage_id = formData.get("stage_id")?.toString();
  const name = formData.get("name")?.toString().trim();
  const sort_order = Number(formData.get("sort_order")) || 0;

  if (!id && !stage_id) return { error: "Stage is required." };
  if (!name) return { error: "Category name is required." };

  if (id) {
    const { error } = await supabase
      .from("cost_categories")
      .update({ name, sort_order })
      .eq("id", id);
    if (error) return { error: describeError(error) };
  } else {
    const { error } = await supabase
      .from("cost_categories")
      .insert({ stage_id, name, sort_order });
    if (error) return { error: describeError(error) };
  }

  refresh();
  return { ok: true };
}

export async function renameCostCategory(id: string, name: string) {
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

  const { error } = await supabase
    .from("cost_categories")
    .update({ name: trimmed })
    .eq("id", id);
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

  refresh();
  return { ok: true, message: `Category renamed to "${trimmed}".` };
}

export async function deleteCostCategory(id: string) {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_categories").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  refresh();
  return { ok: true };
}

export async function saveCostSubcategory(formData: FormData) {
  const supabase = await createClient();
  const id = formData.get("id")?.toString();
  const category_id = formData.get("category_id")?.toString();
  const name = formData.get("name")?.toString().trim();
  const sort_order = Number(formData.get("sort_order")) || 0;

  if (!category_id) return { error: "Category is required." };
  if (!name) return { error: "Subcategory name is required (e.g. Birch, Acacia)." };

  if (id) {
    const { error } = await supabase
      .from("cost_subcategories")
      .update({ name, sort_order })
      .eq("id", id);
    if (error) return { error: describeError(error) };
  } else {
    const { error } = await supabase
      .from("cost_subcategories")
      .insert({ category_id, name, sort_order });
    if (error) return { error: describeError(error) };
  }

  refresh();
  return { ok: true };
}

export async function renameCostSubcategory(id: string, name: string) {
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

  const { error } = await supabase
    .from("cost_subcategories")
    .update({ name: trimmed })
    .eq("id", id);
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

  refresh();
  return { ok: true, message: `Subcategory renamed to "${trimmed}".` };
}

export async function moveCostSubcategory(subcategoryId: string, targetCategoryId: string) {
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

  // Check collision in target category
  const { data: collision } = await supabase
    .from("cost_subcategories")
    .select("id")
    .eq("category_id", targetCategoryId)
    .ilike("name", sub.name)
    .maybeSingle();

  if (collision) {
    return {
      error: `A subcategory named "${sub.name}" already exists in "${targetCat.name}". Please rename it first or choose another category.`,
    };
  }

  // Get max sort order in target category
  const { data: existingSubs } = await supabase
    .from("cost_subcategories")
    .select("sort_order")
    .eq("category_id", targetCategoryId)
    .order("sort_order", { ascending: false })
    .limit(1)
    .returns<{ sort_order: number }[]>();

  const nextSort = (existingSubs?.[0]?.sort_order ?? 0) + 1;

  const { error: moveErr } = await supabase
    .from("cost_subcategories")
    .update({ category_id: targetCategoryId, sort_order: nextSort })
    .eq("id", subcategoryId);

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

  refresh();
  return {
    ok: true,
    message: `Subcategory "${sub.name}" moved to "${targetCat.name}".`,
  };
}

export async function duplicateCostSubcategory(
  subcategoryId: string,
  targetCategoryId: string,
  newName?: string,
) {
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

  // Check collision in target category
  const { data: collision } = await supabase
    .from("cost_subcategories")
    .select("id")
    .eq("category_id", targetCategoryId)
    .ilike("name", trimmedName)
    .maybeSingle();

  if (collision) {
    return {
      error: `A subcategory named "${trimmedName}" already exists in "${targetCat.name}". Please pick a different name.`,
    };
  }

  // Next sort order in target category
  const { data: existingSubs } = await supabase
    .from("cost_subcategories")
    .select("sort_order")
    .eq("category_id", targetCategoryId)
    .order("sort_order", { ascending: false })
    .limit(1)
    .returns<{ sort_order: number }[]>();

  const nextSort = (existingSubs?.[0]?.sort_order ?? 0) + 1;

  // 1. Insert new subcategory
  const { data: newSub, error: subErr } = await supabase
    .from("cost_subcategories")
    .insert({
      category_id: targetCategoryId,
      name: trimmedName,
      sort_order: nextSort,
      is_active: true,
    })
    .select("id")
    .single<{ id: string }>();

  if (subErr || !newSub) return { error: describeError(subErr ?? "Failed to create subcategory") };

  // 2. Insert duplicated varieties
  const vars = varieties ?? [];
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

    const { error: varErr } = await supabase.from("cost_varieties").insert(varietyPayloads);
    if (varErr) {
      await supabase.from("cost_subcategories").delete().eq("id", newSub.id);
      return { error: describeError(varErr) };
    }
  }

  refresh();
  return {
    ok: true,
    message: `Subcategory "${trimmedName}" created in "${targetCat.name}" with ${vars.length} varieties.`,
  };
}

export async function deleteCostSubcategory(id: string) {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_subcategories").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  refresh();
  return { ok: true };
}

export async function saveCostVariety(formData: FormData) {
  const supabase = await createClient();
  const id = formData.get("id")?.toString();
  const subcategory_id = formData.get("subcategory_id")?.toString();
  const name = formData.get("name")?.toString().trim();
  const default_rate = Number(formData.get("default_rate")) || 0;
  const unit = formData.get("unit")?.toString().trim() || "sq ft";
  const default_wastage_pct = Number(formData.get("default_wastage_pct")) || 0;
  const notes = formData.get("notes")?.toString().trim() || null;
  const sort_order = Number(formData.get("sort_order")) || 0;

  if (!subcategory_id) return { error: "Subcategory is required." };
  if (!name) return { error: "Variety name is required (e.g. 8mm, 12mm)." };

  const values = {
    subcategory_id,
    name,
    default_rate,
    unit,
    default_wastage_pct,
    notes,
    sort_order,
  };

  if (!id) {
    const { error } = await supabase.from("cost_varieties").insert(values);
    if (error) return { error: describeError(error) };
    refresh();
    return { ok: true };
  }

  // A new rate or unit reprices every product costed with this variety; a
  // new name is copied onto their lines too.
  const { data: before } = await supabase
    .from("cost_varieties")
    .select("default_rate, unit, name")
    .eq("id", id)
    .maybeSingle<{ default_rate: number; unit: string; name: string }>();
  const repricing =
    !!before && (Number(before.default_rate) !== default_rate || before.unit !== unit || before.name !== name);
  if (repricing) {
    const { profile } = await requireUser();
    if (!isAdmin(profile.role)) {
      return { error: "Only an admin can change a variety's name, rate or unit, because it updates the products that use it." };
    }
  }

  const { error } = await supabase.from("cost_varieties").update(values).eq("id", id);
  if (error) return { error: describeError(error) };
  refresh();
  if (!repricing) return { ok: true };

  const res = await repriceVariety(supabase, id);
  revalidatePath("/products");
  revalidatePath("/products/[...code]", "page");
  if (res.error) {
    return { error: `Rate saved, but repricing stopped after ${res.products} product(s): ${res.error}` };
  }
  return {
    ok: true,
    message: res.products
      ? `Rate saved. ${res.products} product${res.products === 1 ? "" : "s"} repriced; quotations and invoices keep their prices.`
      : "Rate saved. No saved products use this variety yet.",
  };
}

export async function deleteCostVariety(id: string) {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_varieties").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  refresh();
  return { ok: true };
}

/** Saves the colour list and each colour's swatch, used everywhere colours show. */
export async function saveProductColors(colors: ProductColor[]) {
  const supabase = await createClient();
  const seen = new Set<string>();
  const cleaned = colors
    .map((c) => ({ ...c, name: c.name.trim() }))
    .filter((c) => c.name && !seen.has(c.name.toLowerCase()) && seen.add(c.name.toLowerCase()));
  const { error } = await supabase.from("app_settings").upsert(
    [
      { key: "product_colors", value: cleaned.map((c) => c.name) },
      { key: "color_hex", value: Object.fromEntries(cleaned.map((c) => [c.name, c.hex])) },
    ],
    { onConflict: "key" },
  );
  if (error) return { error: describeError(error) };
  refresh();
  revalidatePath("/products");
  return { ok: true };
}
