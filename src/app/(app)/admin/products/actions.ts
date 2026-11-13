"use server";

import { createClient, requireRole } from "@/lib/supabase/server";
import type { ProductWithCategory } from "@/lib/types";

/** Every live product, paged past Supabase's 1000-row cap, for the editable sheet. */
export async function productsForSheet(): Promise<ProductWithCategory[]> {
  await requireRole("admin");
  const supabase = await createClient();
  const all: ProductWithCategory[] = [];

  for (let from = 0; ; from += 1000) {
    const { data, error } = await supabase
      .from("products")
      .select("*, categories(name, counts_as_item)")
      .is("deleted_at", null)
      .order("code")
      .range(from, from + 999)
      .returns<ProductWithCategory[]>();
    if (error) throw new Error(error.message);
    all.push(...data);
    if (data.length < 1000) return all;
  }
}
