import Papa from "papaparse";
import { createClient } from "@/lib/supabase/server";
import type { ProductWithCategory } from "@/lib/types";

/**
 * Product Master as CSV, in the same column order the spreadsheet used, so it
 * round-trips through the importer and still opens cleanly in Sheets/Excel.
 */
export async function GET() {
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) return new Response("Unauthorized", { status: 401 });

  const { data } = await supabase
    .from("products")
    .select("*, categories(name, counts_as_item)")
    .is("deleted_at", null)
    .order("code")
    .overrideTypes<ProductWithCategory[], { merge: false }>();

  const csv = Papa.unparse({
    fields: [
      "Product Code",
      "Category",
      "Product Name",
      "Source",
      "Cost Price",
      "Target Margin %",
      "Default SP",
      "Active",
    ],
    data: (data ?? []).map((p) => [
      p.code,
      p.categories?.name ?? "",
      p.name,
      p.source ?? "",
      p.cost_price,
      p.target_margin,
      p.default_sp,
      p.is_active ? "Yes" : "No",
    ]),
  });

  const stamp = new Date().toISOString().slice(0, 10);

  return new Response("﻿" + csv, {
    headers: {
      "Content-Type": "text/csv; charset=utf-8",
      "Content-Disposition": `attachment; filename="product-master-${stamp}.csv"`,
    },
  });
}
