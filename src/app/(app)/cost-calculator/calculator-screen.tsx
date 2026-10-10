import { notFound } from "next/navigation";
import { requireUser } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { resolveColors } from "@/lib/product-code";
import type { Category, Product } from "@/lib/types";
import { CostCalculatorView } from "./_calculator/calculator-view";
import { loadCostStages, loadProductCosting } from "./load";

/** The calculator, blank or loaded with one product's costing. */
export async function CalculatorScreen({
  code,
  saved,
  t,
}: {
  code?: string;
  /** Codes from the save that reset this page, shown as a banner. */
  saved?: string;
  /** Changes on every save so the page remounts even on the same URL. */
  t?: string;
}) {
  const { supabase } = await requireUser();
  const [stages, { data: products }, { data: categories }, settings, costing] = await Promise.all([
    loadCostStages(supabase),
    // Only the product picker reads these.
    supabase.from("products").select("id, code, name").is("deleted_at", null).order("code").returns<Pick<Product, "id" | "code" | "name">[]>(),
    supabase.from("categories").select("*").order("sort_order").returns<Category[]>(),
    loadSettings(supabase),
    code ? loadProductCosting(supabase, code) : null,
  ]);
  if (code && !costing) notFound();

  return (
    <CostCalculatorView
      // Fresh state (photos, lines) when switching to another product
      key={`${costing?.product.id ?? "new"}-${t ?? ""}`}
      stages={stages}
      categories={categories ?? []}
      products={products ?? []}
      productColors={resolveColors(settings.product_colors, settings.color_hex)}
      initialProduct={costing?.product ?? null}
      initialSheet={costing?.sheet ?? null}
      initialImages={costing?.images ?? []}
      initialVariantIds={costing?.variantIds ?? {}}
      savedCodes={saved}
    />
  );
}
