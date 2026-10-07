import Link from "@/components/link";
import { notFound, redirect } from "next/navigation";
import { requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { resolveColors } from "@/lib/product-code";
import { calculateCostSheetTotals } from "@/lib/costing.ts";
import { num } from "@/lib/pricing";
import { PageHeader } from "@/components/ui";
import type { Category } from "@/lib/types";
import { costingHref } from "../../cost-calculator/href";
import { loadCostStages, loadProductCosting } from "../../cost-calculator/load";
import { CostTable } from "./_view/cost-table";
import { PriceSummary } from "./_view/price-summary";
import { ProductCard } from "./_view/product-card";

/** A product's saved costing in the calculator's layout, read-only; Edit opens the calculator. */
export default async function ProductPage({
  params,
  searchParams,
}: {
  params: Promise<{ code: string | string[] }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { code } = await params;
  const decodedCode = decodeURIComponent(Array.isArray(code) ? code.join("/") : code);
  // Products are edited in the cost calculator; old ?edit=1 links go there.
  if ((await searchParams).edit === "1") redirect(costingHref(decodedCode));

  const { supabase } = await requireRole("admin");
  const [costing, stages, settings, { data: categories }] = await Promise.all([
    loadProductCosting(supabase, decodedCode),
    loadCostStages(supabase),
    loadSettings(supabase),
    supabase.from("categories").select("*").returns<Category[]>(),
  ]);
  if (!costing) notFound();

  const { product, sheet, images } = costing;
  const lines = sheet?.lines ?? [];
  const overheads = sheet?.stage_overheads ?? {};
  const totals = sheet ? calculateCostSheetTotals(lines, sheet.markup_pct, overheads) : null;
  // Cost types in the calculator's order, skipping any with no lines; the rest is bought out.
  const sections = stages.filter((s) => lines.some((l) => l.stage_code === s.code));
  const known = new Set(stages.map((s) => s.code));
  const boughtOut = lines.filter((l) => !known.has(l.stage_code));

  return (
    <>
      <PageHeader title={product.name} subtitle={product.code}>
        <Link href={costingHref(product.code)} className="btn-secondary">
          Edit
        </Link>
        <Link href="/products/new" className="btn-primary">
          Add new product
        </Link>
      </PageHeader>

      <div className="space-y-6">
        <ProductCard
          product={product}
          categoryName={categories?.find((c) => c.id === product.category_id)?.name ?? null}
          colors={resolveColors(product.colors ?? [], settings.color_hex)}
          photos={images}
        />

        {!sheet && (
          <div className="card border-dashed p-4 text-center text-sm text-(--color-muted)">
            No costing saved for this product yet. Click <strong>Edit</strong> to cost it in the calculator.
          </div>
        )}

        {sections.map((s) => (
          <CostTable
            key={s.id}
            badge={String(s.sort_order)}
            title={s.name}
            lines={lines.filter((l) => l.stage_code === s.code)}
            overhead={num(overheads[s.code.toLowerCase()])}
          />
        ))}
        {boughtOut.length > 0 && <CostTable badge="B" title="Bought Out Items" lines={boughtOut} />}

        <PriceSummary
          totals={totals}
          costPrice={product.cost_price}
          markupPct={sheet?.markup_pct ?? product.markup_pct}
          sellingPrice={product.default_sp}
          notes={sheet?.notes}
        />
      </div>
    </>
  );
}
