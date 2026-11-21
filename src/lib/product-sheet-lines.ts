/**
 * The Cost Lines and Rates tabs of the Product Master workbook. A line's Unit,
 * Rate and Wastage % look its item up on the Rates tab, and Line Total works the
 * line out the way calculateLineCost() does, so Excel shows costs as you type.
 * The upload works everything out again; Excel's figures are only a preview.
 * Each product's lines are a block, with a blank row before the next.
 */
import { calculateLineCost } from "./costing.ts";
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { LINE_COL, LINE_HEADERS, LINES_SHEET, RATES_SHEET, SHEET_NAME, stageLabel, type LineHeader } from "./product-sheet-costing.ts";
import type { CostVariety, ProductCostLine } from "./types";
import { formulaRow, type Cell, type Column, type Formula, type Sheet } from "./xlsx-sheet.ts";

export const letter = (i: number) => String.fromCharCode(65 + i);
const at = (h: LineHeader, r: number) => `${letter(LINE_COL[h])}${r}`;

// Pricing units as minutes (time), or as factors on inches (lengths) and square inches (areas).
const table = (rows: [string[], number][]) => rows.flatMap(([names, f]) => names.map((n) => `"${n}",${f}`)).join(";");
const TIME = table([[["min", "mins", "minute", "minutes"], 1], [["hour", "hr", "hours"], 60]]);
const LINEAR = table([[["feet", "ft", "running ft", "rft"], 1 / 12], [["inch", "in"], 1], [["mm"], 25.4], [["cm"], 2.54], [["meter", "m"], 0.0254]]);
const AREA = table([[["sq ft", "sqft", "sft"], 1 / 144], [["sq inch", "sqinch", "sq in", "sqin"], 1], [["sq mm", "sqmm"], 645.16], [["sq cm", "sqcm"], 6.4516]]);

/** Rates tab: one row per variety, with the key the Cost Lines look it up by. */
const RATE_COLUMNS: Column[] = [
  { header: "Type", width: 12 },
  { header: "Category", width: 20 },
  { header: "Subcategory", width: 20 },
  { header: "Variety", width: 22 },
  { header: "Unit", width: 10 },
  { header: "Rate", width: 11, format: "money" },
  { header: "Wastage %", width: 11 },
  { header: "Lookup key", width: 40, formula: (r) => `TRIM(A${r})&"|"&TRIM(B${r})&"|"&TRIM(C${r})&"|"&TRIM(D${r})` },
];

const key = (r: number) => ["Type", "Category", "Subcategory", "Variety"].map((h) => `TRIM(${at(h as LineHeader, r)})`).join(`&"|"&`);
/** Unit (E), Rate (F) or Wastage % (G) of the line's item on the Rates tab; blank when it isn't there. */
const lookup = (col: "E" | "F" | "G") => (r: number) =>
  `IF(${at("Product Code", r)}="","",IFERROR(INDEX(${RATES_SHEET}!$${col}:$${col},MATCH(${key(r)},${RATES_SHEET}!$H:$H,0)),""))`;

function lineTotal(r: number) {
  const type = `LOWER(TRIM(${at("Type", r)}))`;
  const machine = `LEFT(${type},7)="machine"`;
  const u = `IF(TRIM(${at("Unit", r)})="",IF(${machine},"min","sq ft"),LOWER(TRIM(${at("Unit", r)})))`;
  const factor = (rows: string) => `IFERROR(VLOOKUP(${u},{${rows}},2,FALSE),0)`;
  const inches = (h: LineHeader) => `N(${at(h, r)})/IF(${at("Size In", r)}="mm",25.4,IF(${at("Size In", r)}="cm",2.54,1))`;
  const each = `N(${at("Rate", r)})*IF(${at("Qty", r)}="",1,N(${at("Qty", r)}))`;
  const [len, wide] = [`N(${at("Length", r)})>0`, `N(${at("Breadth", r)})>0`];
  const base =
    `IF(AND(${machine},${factor(TIME)}>0),N(${at("Minutes", r)})/${factor(TIME)}*${each},` +
    `IF(AND(${factor(LINEAR)}>0,${len}),${inches("Length")}*${factor(LINEAR)}*${each},` +
    `IF(AND(${factor(AREA)}>0,${len},${wide}),${inches("Length")}*${inches("Breadth")}*${factor(AREA)}*${each},${each})))`;
  const wastage = `IF(LEFT(${type},6)="bought",0,N(${at("Wastage %", r)})/100)`;
  return `IF(${at("Product Code", r)}="","",ROUND(${base}*(1+${wastage}),2))`;
}

/** The product's name, from its row on the Product Master tab. */
const productName = (r: number) =>
  `IF(${at("Product Code", r)}="","",IFERROR(INDEX('${SHEET_NAME}'!$C:$C,MATCH(${at("Product Code", r)},'${SHEET_NAME}'!$A:$A,0)),""))`;

const FORMULAS: Partial<Record<LineHeader, (r: number) => string>> = {
  "Product Name": productName,
  Unit: lookup("E"),
  Rate: lookup("F"),
  "Wastage %": lookup("G"),
  "Line Total": lineTotal,
};
const WIDE: Partial<Record<LineHeader, number>> = { "Product Code": 16, "Product Name": 30, Category: 18, Subcategory: 18, Variety: 20 };
const LINE_COLUMNS: Column[] = LINE_HEADERS.map((header) => ({
  header,
  width: WIDE[header] ?? 10,
  format: header === "Rate" || header === "MRP" || header === "Line Total" ? "money" : undefined,
  formula: FORMULAS[header],
}));

type Item = { type: string; category: string; subcategory: string; v: CostVariety };

/** Every variety in the master, with its Type as the sheet spells it. */
function masterItems(rows: CostRows) {
  const items: Item[] = [];
  const types = new Map<string, string>();
  for (const stage of buildCostTree(rows)) {
    const type = stageLabel(stage.code, stage.name);
    types.set(stage.code, type);
    for (const c of stage.categories)
      for (const s of c.subcategories) for (const v of s.varieties) items.push({ type, category: c.name, subcategory: s.name, v });
  }
  return { items, types, byId: new Map(items.map((i) => [i.v.id, i])) };
}

/** A blank row before each new subcategory, as on the Rates & Hierarchy sheet. Reference only, so no formulas there. */
export function ratesSheet(rows: CostRows): Sheet {
  const out: Cell[][] = [];
  let last = "";
  for (const { type, category, subcategory, v } of masterItems(rows).items) {
    const sub = `${type}|${category}|${subcategory}`;
    if (out.length && sub !== last) out.push([]);
    last = sub;
    out.push([
      type, category, subcategory, v.name, v.unit, Number(v.default_rate), Number(v.default_wastage_pct),
      { f: RATE_COLUMNS[7].formula!(out.length + 2), v: [type, category, subcategory, v.name].map((s) => s.trim()).join("|") },
    ]);
  }
  return { name: RATES_SHEET, columns: RATE_COLUMNS, blankRows: 0, rows: out };
}

/**
 * Every product's lines, brought up to the master's current rates as the
 * calculator shows them, and each product's line total per stage.
 */
export function linesSheet(products: { code: string; name: string; lines: ProductCostLine[] }[], rows: CostRows) {
  const { types, byId } = masterItems(rows);
  const out: Cell[][] = [];
  const sums = new Map<string, Record<string, number>>();
  for (const { code, name, lines } of products) {
    const sum: Record<string, number> = {};
    // A blank row between products, carrying the formulas so it can be typed into.
    if (out.length && lines.length) out.push(formulaRow(LINE_COLUMNS, out.length + 2));
    for (const saved of [...lines].sort((a, b) => Number(a.sort_order) - Number(b.sort_order))) {
      const m = saved.cost_variety_id ? byId.get(saved.cost_variety_id) : undefined;
      const l: ProductCostLine = m
        ? { ...saved, category_name: m.category, subcategory_name: m.subcategory, variety_name: m.v.name, rate: Number(m.v.default_rate), unit: m.v.unit, wastage_pct: Number(m.v.default_wastage_pct) }
        : saved;
      const total = calculateLineCost(l).line_total;
      sum[l.stage_code] = (sum[l.stage_code] ?? 0) + total;
      const n = out.length + 2;
      const fromRates = (col: "E" | "F" | "G", v: string | number): Cell => (m ? ({ f: lookup(col)(n), v } satisfies Formula) : v);
      out.push([
        code, { f: productName(n), v: name }, types.get(l.stage_code) ?? stageLabel(l.stage_code), l.category_name, l.subcategory_name ?? "", l.variety_name ?? "",
        l.length ?? "", l.breadth ?? "", l.dimension_unit || "inch", l.qty ?? 1, l.duration_minutes ?? "",
        fromRates("E", l.unit), fromRates("F", Number(l.rate)), fromRates("G", Number(l.wastage_pct)),
        Number(l.mrp) > 0 ? Number(l.mrp) : "", { f: lineTotal(n), v: total },
      ]);
    }
    sums.set(code, sum);
  }
  return { sheet: { name: LINES_SHEET, columns: LINE_COLUMNS, rows: out, blankRows: 1000 } satisfies Sheet, sums };
}
