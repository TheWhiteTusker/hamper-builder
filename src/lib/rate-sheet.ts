/**
 * The Rates & Hierarchy Master as one flat sheet: a row per variety, with its
 * type (stage), category and subcategory spelled out. Read back by
 * planRateImport() in rate-import.ts (kept apart so the server never loads SheetJS).
 */
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { buildXlsx, formulaRow, type Cell, type Column } from "./xlsx-sheet.ts";

export const RATE_SHEET_NAME = "Rates & Hierarchy";

const COLUMNS: Column[] = [
  { header: "Type", width: 16 },
  { header: "Category", width: 22 },
  { header: "Subcategory", width: 22 },
  { header: "Variety / Specification", width: 26 },
  { header: "Rate", width: 12, format: "money" },
  { header: "Unit", width: 10 },
  { header: "Wastage %", width: 11 },
  { header: "Notes", width: 40 },
  // Display only; the importer reads columns A–H.
  { header: "Rate incl. wastage", width: 18, format: "money", formula: (r) => `IF(E${r}="","",E${r}*(1+N(G${r})/100))` },
];

/** A row per variety; a blank row (still carrying the formula) before each new subcategory. */
export function rateRows(db: CostRows): Cell[][] {
  const rows: Cell[][] = [];
  for (const stage of buildCostTree(db))
    for (const cat of stage.categories)
      for (const sub of cat.subcategories) {
        if (rows.length && sub.varieties.length) rows.push(formulaRow(COLUMNS, rows.length + 2));
        for (const v of sub.varieties) {
          const [rate, wastage] = [Number(v.default_rate), Number(v.default_wastage_pct)];
          rows.push([
            stage.name, cat.name, sub.name, v.name, rate, v.unit, wastage, v.notes ?? "",
            { f: COLUMNS[8].formula!(rows.length + 2), v: rate * (1 + wastage / 100) },
          ]);
        }
      }
  return rows;
}

export const buildRateSheet = (rows: Cell[][] = []) => buildXlsx(RATE_SHEET_NAME, COLUMNS, rows);
