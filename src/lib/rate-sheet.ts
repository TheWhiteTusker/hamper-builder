/**
 * The Rates & Hierarchy Master as one flat sheet: a row per variety, with its
 * type (stage), category and subcategory spelled out. Read back by
 * planRateImport() in rate-import.ts (kept apart so the server never loads SheetJS).
 */
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { buildXlsx, type Cell, type Column } from "./xlsx-sheet.ts";

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

export function rateRows(db: CostRows): Cell[][] {
  return buildCostTree(db)
    .flatMap((stage) =>
      stage.categories.flatMap((cat) =>
        cat.subcategories.flatMap((sub) => sub.varieties.map((v) => ({ stage, cat, sub, v }))),
      ),
    )
    .map(({ stage, cat, sub, v }, i) => {
      const rate = Number(v.default_rate);
      const wastage = Number(v.default_wastage_pct);
      return [
        stage.name, cat.name, sub.name, v.name, rate, v.unit, wastage, v.notes ?? "",
        { f: COLUMNS[8].formula!(i + 2), v: rate * (1 + wastage / 100) },
      ];
    });
}

export const buildRateSheet = (rows: Cell[][] = []) => buildXlsx(RATE_SHEET_NAME, COLUMNS, rows);
