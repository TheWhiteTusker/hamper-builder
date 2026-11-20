/**
 * The costing half of the Product Master workbook: the Cost Lines tab (one row
 * per line, as in the calculator) and the overhead % per stage on the Product
 * Master tab. Reads a product's lines back and works out what changes.
 * Pure (no SheetJS): read on the server, laid out here for product-sheet*.ts.
 */
import { calculateCostSheetTotals, calculateLineCost, type CostSheetTotals } from "./costing.ts";
import { cell, numbered, type Grid } from "./import-cells.ts";
import { normalizePct, num, round2 } from "./numbers.ts";
import type { CostStageWithHierarchy, ProductCostLine } from "./types";

export const SHEET_NAME = "Product Master";
export const LINES_SHEET = "Cost Lines";
export const RATES_SHEET = "Rates";

export const SHEET_STAGES = [
  { code: "material", label: "Material" },
  { code: "hardware", label: "Hardware" },
  { code: "finishing", label: "Finishing" },
  { code: "machine", label: "Machine" },
  { code: "miscellaneous", label: "Misc" },
  { code: "bought_out", label: "Bought Out" },
] as const;

/** Product Master column I: each stage takes two columns, its lines' total then its overhead %. */
export const STAGE_COL = 8;

/**
 * Cost Lines columns, in order. Product Name comes from the Product Master tab;
 * Unit, Rate and Wastage % from the Rates tab when the item is on it.
 */
export const LINE_HEADERS = [
  "Product Code", "Product Name", "Type", "Category", "Subcategory", "Variety", "Length", "Breadth", "Size In",
  "Qty", "Minutes", "Unit", "Rate", "Wastage %", "MRP", "Line Total",
] as const;
export type LineHeader = (typeof LINE_HEADERS)[number];
export const LINE_COL = Object.fromEntries(LINE_HEADERS.map((h, i) => [h, i])) as Record<LineHeader, number>;

export const stageLabel = (code: string, name = code) => SHEET_STAGES.find((s) => s.code === code)?.label ?? name;

const norm = (s: string | null | undefined) => (s ?? "").trim().toLowerCase();
const numOrNull = (v: string) => (v === "" ? null : num(v));
// "10.00%" is 10; a bare 0.1 or 10 is read the way Markup % is.
const pct = (v: string) => round2(v.endsWith("%") ? num(v) : normalizePct(v) * 100);

/** The Product Master row's overhead % per stage; blank and 0 are left out. */
export function readOverheads(row: string[]): Record<string, number> {
  const out: Record<string, number> = {};
  SHEET_STAGES.forEach(({ code }, i) => {
    const v = cell(row, STAGE_COL + 2 * i + 1);
    if (v && pct(v)) out[code] = pct(v);
  });
  return out;
}

/**
 * The Cost Lines tab as each product's lines. An item on the Rates page takes
 * its rate and unit from there, as the calculator does; one that is not keeps
 * what was typed, with a warning. An unknown Type is an error.
 * A wastage % typed over a Rates item's is a change to the Rates page: it goes
 * into `wastage` (variety id -> new %) and onto every line of that item.
 */
export function readCostLines(
  grid: Grid | undefined,
  stages: CostStageWithHierarchy[],
  warnings: string[],
  errors: string[],
  wastage = new Map<string, number>(),
) {
  const stageBy = new Map(stages.flatMap((s) => [norm(s.code), norm(s.name), norm(stageLabel(s.code))].map((k) => [k, s] as const)));
  const lines = new Map<string, ProductCostLine[]>();
  const labels = new Map<string, [string, number]>();
  numbered(grid).forEach(({ row, n }) => {
    const get = (h: LineHeader) => cell(row, LINE_COL[h]);
    const code = get("Product Code");
    if (!code || (!get("Type") && !get("Category") && !get("Variety"))) return;
    const stage = stageBy.get(norm(get("Type")));
    if (!stage) {
      errors.push(`Cost Lines row ${n}: Type "${get("Type")}" isn't one of ${SHEET_STAGES.map((s) => s.label).join(", ")}.`);
      return;
    }
    const cat = stage.categories.find((c) => norm(c.name) === norm(get("Category")));
    const sub = cat?.subcategories.find((s) => norm(s.name) === norm(get("Subcategory")));
    const v = sub?.varieties.find((x) => norm(x.name) === norm(get("Variety")));
    const subName = sub?.name ?? get("Subcategory");
    const varName = v?.name ?? get("Variety");
    const bought = stage.code === "bought_out";
    const size = norm(get("Size In"));
    const line: ProductCostLine = {
      stage_code: stage.code,
      category_name: cat?.name ?? get("Category"),
      subcategory_name: subName || null,
      variety_name: varName || null,
      item_name: [subName, varName].filter(Boolean).join(" ") || get("Category"),
      cost_variety_id: v?.id ?? null,
      length: numOrNull(get("Length")),
      breadth: numOrNull(get("Breadth")),
      dimension_unit: size === "mm" || size === "cm" ? size : "inch",
      unit: v?.unit ?? (get("Unit") || (stage.code === "machine" ? "min" : "piece")),
      rate: v ? Number(v.default_rate) : num(get("Rate")),
      duration_minutes: numOrNull(get("Minutes")),
      qty: get("Qty") === "" ? 1 : num(get("Qty")),
      wastage_pct: bought ? 0 : v ? Number(v.default_wastage_pct) : num(get("Wastage %")),
      mrp: num(get("MRP")) > 0 ? num(get("MRP")) : null,
      line_total: 0,
    };
    const path = [get("Type"), get("Category"), get("Subcategory"), get("Variety")].filter(Boolean).join(" › ");
    if (!v) warnings.push(`${code}: ${path} isn't on the Rates page, so its typed rate ₹${line.rate} is used.`);
    // Untouched rows still show the Rates value, so only a different one is an edit.
    const typed = get("Wastage %");
    if (v && !bought && typed !== "" && round2(num(typed)) !== Number(v.default_wastage_pct)) {
      const w = round2(num(typed));
      const seen = wastage.get(v.id);
      if (seen != null && seen !== w) errors.push(`Cost Lines row ${n}: ${path} has wastage ${w}% here but ${seen}% on another row; use one.`);
      wastage.set(v.id, w);
      labels.set(v.id, [path, Number(v.default_wastage_pct)]);
    }
    const list = lines.get(code) ?? [];
    list.push({ ...line, sort_order: list.length + 1 });
    lines.set(code, list);
  });
  for (const [id, w] of wastage) {
    const [path, was] = labels.get(id)!;
    warnings.push(`${path}: wastage ${was}% → ${w}% on the Rates page; every product using it is repriced.`);
  }
  for (const list of lines.values()) {
    list.forEach((l, i) => {
      const w = l.cost_variety_id ? wastage.get(l.cost_variety_id) : undefined;
      const line = w == null || l.stage_code === "bought_out" ? l : { ...l, wastage_pct: w };
      list[i] = { ...line, ...calculateLineCost(line) };
    });
  }
  return lines;
}

export type ExistingCosting = { sheetId: string; overheads: Record<string, number>; lines: ProductCostLine[] };

export type CostingPlan = {
  code: string;
  sheetId?: string;
  overheads: Record<string, number>;
  totals: CostSheetTotals;
  lines: ProductCostLine[];
};

// What a line is, ignoring ids and order numbers, so a round trip compares equal.
const sig = (l: ProductCostLine) =>
  [
    l.stage_code, l.category_name, l.subcategory_name ?? "", l.variety_name ?? "", l.cost_variety_id ?? "",
    num(l.length), num(l.breadth), l.dimension_unit || "inch", l.unit, num(l.rate), num(l.duration_minutes),
    l.qty == null ? 1 : num(l.qty), num(l.wastage_pct), num(l.mrp),
  ].join("|");
const sameOverheads = (a: Record<string, number>, b: Record<string, number>) => {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].every((k) => num(a[k]) === num(b[k]));
};

/**
 * A product's costing from the sheet: its lines replace the saved ones, with
 * the row's overheads. Null when that is what is saved already.
 */
export function planSheetCosting(
  p: { code: string; cost_price: number; markup_pct?: number },
  lines: ProductCostLine[],
  overheads: Record<string, number>,
  existing: ExistingCosting | undefined,
  warnings: string[],
): CostingPlan | null {
  const saved = [...(existing?.lines ?? [])].sort((a, b) => num(a.sort_order) - num(b.sort_order));
  if (existing && sameOverheads(overheads, existing.overheads) && saved.map(sig).join("\n") === lines.map(sig).join("\n")) {
    return null;
  }
  const totals = calculateCostSheetTotals(lines, p.markup_pct ?? 0, overheads);
  // Excel works the same sums out; a gap means a typed-over Cost Price or a rate the Rates page changed.
  if (Math.abs(p.cost_price - totals.total_cost) >= 0.5) {
    warnings.push(`${p.code}: its lines cost ₹${totals.total_cost}, not the ₹${p.cost_price} in Cost Price; ₹${totals.total_cost} is saved.`);
  }
  return { code: p.code, sheetId: existing?.sheetId, overheads, totals, lines };
}
