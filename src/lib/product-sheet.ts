/**
 * Product Master as an editable .xlsx. Tab 1 is one row per product, in the
 * column order buildImport() reads (a blank row between products, none between
 * one product's colours), then per stage the total of its lines and
 * an overhead %. Tab 2 (Cost Lines) holds every product's lines, tab 3 the Rates.
 * Cost Price adds up the stages; Default SP is the cost calculator's rule (costing.ts):
 * SP = Cost ÷ (1 − Markup %), rounded up to the next 10; 100%+ falls back to cost.
 */
import { round2, roundUpToNext10 } from "./numbers.ts";
import type { CostRows } from "./cost-tree.ts";
import { LINE_COL, LINES_SHEET, SHEET_NAME, SHEET_STAGES, STAGE_COL } from "./product-sheet-costing.ts";
import { letter, linesSheet, ratesSheet } from "./product-sheet-lines.ts";
import { parseProductCode } from "./product-code.ts";
import { buildWorkbook, formulaRow, type Cell, type Column } from "./xlsx-sheet.ts";
import type { ProductCostLine, ProductWithCategory } from "./types.ts";

export { SHEET_NAME };

/** A product with its cost sheet's overheads and lines, for the download. */
export type SheetProduct = ProductWithCategory & {
  product_cost_sheets?: { stage_overheads: Record<string, number> | null; product_cost_lines: ProductCostLine[] }[];
};

const L = `'${LINES_SHEET}'!`;
const [codes, types, totals] = (["Product Code", "Type", "Line Total"] as const).map((h) => `${L}$${letter(LINE_COL[h])}:$${letter(LINE_COL[h])}`);
const stageCells = (r: number) =>
  SHEET_STAGES.map((_, i) => [`${letter(STAGE_COL + 2 * i)}${r}`, `${letter(STAGE_COL + 2 * i + 1)}${r}`]);

/** The product's Cost Lines of one Type, added up; blank when it has none. */
const stageFormula = (label: string) => (r: number) =>
  `IF(COUNTIFS(${codes},$A${r},${types},"${label}")=0,"",SUMIFS(${totals},${codes},$A${r},${types},"${label}"))`;

// Each stage rounded on its own, as calculateCostSheetTotals does. Blank without lines.
const costFormula = (r: number) =>
  `IF(OR($A${r}="",COUNTIF(${codes},$A${r})=0),"",${stageCells(r).map(([a, o]) => `ROUND(N(${a})*(1+N(${o})),2)`).join("+")})`;

const spFormula = (r: number) =>
  `IF(E${r}="","",IF(N(F${r})>=1,CEILING(E${r},10),CEILING(ROUND(E${r}/(1-N(F${r})),2),10)))`;

/** What spFormula works out to, for a markup given as a fraction. */
const formulaSp = (cp: number, m: number) => roundUpToNext10(m >= 1 ? cp : round2(cp / (1 - m)));

const COLUMNS: Column[] = [
  { header: "Product Code", width: 16 },
  { header: "Category", width: 20 },
  { header: "Product Name", width: 40 },
  { header: "Source", width: 18 },
  { header: "Cost Price", width: 12, format: "money", formula: costFormula },
  { header: "Markup %", width: 11, format: "percent" },
  { header: "Selling Price", width: 13, format: "money", formula: spFormula },
  { header: "Active", width: 9 },
  ...SHEET_STAGES.flatMap(({ label }): Column[] => [
    { header: label, width: 12, format: "money", formula: stageFormula(label) },
    { header: `${label} OH %`, width: 11, format: "percent" },
  ]),
];

/** `r` is the sheet row the product lands on (2 for the first); `sums` its line totals per stage. */
export function productRow(p: SheetProduct, r: number, sums: Record<string, number> = {}): Cell[] {
  const cp = Number(p.cost_price);
  const sp = Number(p.default_sp);
  // The stored markup, else the one implied by today's price (older products have none).
  const stored = Number(p.markup_pct ?? 0) / 100;
  const m = stored > 0 ? stored : sp > 0 ? (sp - cp) / sp : 0;
  const overheads = p.product_cost_sheets?.[0]?.stage_overheads ?? {};
  const stages = SHEET_STAGES.map(({ code, label }) => ({
    label,
    amount: code in sums ? round2(sums[code]) : null,
    overhead: Number(overheads[code] ?? 0),
  }));
  // Formulas are written only where they give back today's figure; otherwise the
  // value stays typed, so downloading and uploading never moves a cost or price.
  const linesCost = round2(stages.reduce((s, { amount, overhead }) => s + round2((amount ?? 0) * (1 + overhead / 100)), 0));
  const liveCost = Object.keys(sums).length > 0 && linesCost === round2(cp);
  const live = cp > 0 && sp > 0 && formulaSp(cp, m) === sp;
  return [
    p.code,
    p.categories?.name ?? "",
    p.name,
    p.source ?? "",
    liveCost ? { f: costFormula(r), v: cp } : cp,
    m || "", // unrounded, so Excel does the same arithmetic as formulaSp
    live ? { f: spFormula(r), v: sp } : sp,
    p.is_active ? "Yes" : "No",
    ...stages.flatMap(({ label, amount, overhead }) => [
      { f: stageFormula(label)(r), v: amount ?? "" },
      overhead ? overhead / 100 : "",
    ]),
  ];
}

const NO_RATES: CostRows = { stages: [], categories: [], subcategories: [], varieties: [] };

export function buildProductSheet(products: SheetProduct[] = [], rates: CostRows = NO_RATES) {
  const lines = linesSheet(
    products.map((p) => ({ code: p.code, name: p.name, lines: p.product_cost_sheets?.[0]?.product_cost_lines ?? [] })),
    rates,
  );
  // A blank row between products; a product's colours (LC/0001/WL, LC/0001/BL) stay together.
  const rows: Cell[][] = [];
  let family = "";
  for (const p of products) {
    const { isValid, categoryCode, serial } = parseProductCode(p.code);
    const next = isValid ? `${categoryCode}/${serial}` : p.code;
    if (rows.length && next !== family) rows.push(formulaRow(COLUMNS, rows.length + 2));
    family = next;
    rows.push(productRow(p, rows.length + 2, lines.sums.get(p.code)));
  }
  return buildWorkbook([
    { name: SHEET_NAME, columns: COLUMNS, rows },
    lines.sheet,
    ratesSheet(rates),
  ]);
}
