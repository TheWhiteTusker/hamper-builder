"use server";

import { revalidatePath } from "next/cache";
import { createClient, isAdmin, requireUser } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { CostSaveResult } from "@/lib/cost-tree";
import type { ProductColor } from "@/lib/product-code";
import type { CostVariety } from "@/lib/types";
import { repriceVarieties } from "../reprice";

// Return the changed rows instead of revalidatePath; see category-actions.ts.
// Repricing is the exception: it changes prices on pages all over the app.

export async function saveCostVariety(formData: FormData): Promise<CostSaveResult> {
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
    const { data, error } = await supabase.from("cost_varieties").insert(values).select().single<CostVariety>();
    if (error) return { error: describeError(error) };
    return { ok: true, changes: [{ table: "varieties", upsert: [data] }] };
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

  const { data: saved, error } = await supabase
    .from("cost_varieties")
    .update(values)
    .eq("id", id)
    .select()
    .single<CostVariety>();
  if (error) return { error: describeError(error) };
  const changes = [{ table: "varieties" as const, upsert: [saved] }];
  if (!repricing) return { ok: true, changes };

  const res = await repriceVarieties(supabase, [id]);
  revalidatePath("/cost-calculator", "layout");
  revalidatePath("/products");
  revalidatePath("/products/[...code]", "page");
  if (res.error) {
    return { error: `Rate saved, but repricing failed: ${res.error}`, changes };
  }
  return {
    ok: true,
    changes,
    message: res.products
      ? `Rate saved. ${res.products} product${res.products === 1 ? "" : "s"} repriced; quotations and invoices keep their prices.`
      : "Rate saved. No saved products use this variety yet.",
  };
}

export async function deleteCostVariety(id: string): Promise<CostSaveResult> {
  const supabase = await createClient();
  const { error } = await supabase.from("cost_varieties").delete().eq("id", id);
  if (error) return { error: describeError(error) };
  return { ok: true, changes: [{ table: "varieties", remove: [id] }] };
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
  // ColorsCard keeps its own list; it marks other pages stale itself.
  return { ok: true };
}
