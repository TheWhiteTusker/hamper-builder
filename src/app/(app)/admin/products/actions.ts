"use server";

import { createClient, requireRole } from "@/lib/supabase/server";
import type { SheetProduct } from "@/lib/product-sheet";

/** Every live product, paged past Supabase's 1000-row cap, for the editable sheet. */
export async function productsForSheet(): Promise<SheetProduct[]> {
  await requireRole("admin");
  const supabase = await createClient();
  const all: SheetProduct[] = [];

  for (let from = 0; ; from += 1000) {
    const { data, error } = await supabase
      .from("products")
      .select("*, categories(name, counts_as_item), product_cost_sheets(stage_overheads, product_cost_lines(*))")
      .is("deleted_at", null)
      .order("code")
      .range(from, from + 999)
      .overrideTypes<SheetProduct[], { merge: false }>();
    if (error) throw new Error(error.message);
    all.push(...data);
    if (data.length < 1000) return all;
  }
}
