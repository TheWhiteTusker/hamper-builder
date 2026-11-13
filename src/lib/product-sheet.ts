/**
 * Product Master as an editable .xlsx, in the column order buildImport() reads.
 * Default SP is a formula, the cost calculator's rule (costing.ts):
 * SP = Cost ÷ (1 − Markup %), rounded up to the next 10; 100%+ falls back to cost.
 */
import { round2, roundUpToNext10 } from "./numbers.ts";
import { buildXlsx, type Cell, type Column } from "./xlsx-sheet.ts";
import type { ProductWithCategory } from "./types.ts";

export const SHEET_NAME = "Product Master";

const spFormula = (r: number) =>
  `IF(E${r}="","",IF(N(F${r})>=1,CEILING(E${r},10),CEILING(ROUND(E${r}/(1-N(F${r})),2),10)))`;

/** What spFormula works out to, for a markup given as a fraction. */
const formulaSp = (cp: number, m: number) => roundUpToNext10(m >= 1 ? cp : round2(cp / (1 - m)));

const COLUMNS: Column[] = [
  { header: "Product Code", width: 16 },
  { header: "Category", width: 20 },
  { header: "Product Name", width: 40 },
  { header: "Source", width: 18 },
  { header: "Cost Price", width: 12, format: "money" },
  { header: "Markup %", width: 11, format: "percent" },
  { header: "Default SP", width: 12, format: "money", formula: spFormula },
  { header: "Active", width: 9 },
];

/** `r` is the sheet row the product lands on (2 for the first). */
export function productRow(p: ProductWithCategory, r: number): Cell[] {
  const cp = Number(p.cost_price);
  const sp = Number(p.default_sp);
  // The stored markup, else the one implied by today's price (older products have none).
  const stored = Number(p.markup_pct ?? 0) / 100;
  const m = stored > 0 ? stored : sp > 0 ? (sp - cp) / sp : 0;
  // Only a formula that gives back today's SP is written; otherwise the SP stays a
  // typed value, so downloading and uploading never moves a price.
  const live = cp > 0 && sp > 0 && formulaSp(cp, m) === sp;
  return [
    p.code,
    p.categories?.name ?? "",
    p.name,
    p.source ?? "",
    cp,
    m || "", // unrounded, so Excel does the same arithmetic as formulaSp
    live ? { f: spFormula(r), v: sp } : sp,
    p.is_active ? "Yes" : "No",
  ];
}

export const buildProductSheet = (products: ProductWithCategory[] = []) =>
  buildXlsx(SHEET_NAME, COLUMNS, products.map((p, i) => productRow(p, i + 2)));
