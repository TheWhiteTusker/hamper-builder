import { test } from "node:test";
import assert from "node:assert/strict";
import * as XLSX from "xlsx";
import { buildProductSheet, SHEET_NAME } from "./product-sheet.ts";
import { buildImport } from "./import.ts";
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { LINES_SHEET, planSheetCosting, RATES_SHEET, readCostLines, readOverheads } from "./product-sheet-costing.ts";
import type { ProductCostLine, ProductWithCategory } from "./types.ts";

const RATES = {
  stages: [
    { id: "st1", code: "material", name: "Material", calculation_type: "dimension", sort_order: 1, is_active: true },
    { id: "st2", code: "machine", name: "Machine Cost", calculation_type: "time_based", sort_order: 2, is_active: true },
  ],
  categories: [{ id: "c1", stage_id: "st1", name: "Woodbased", sort_order: 0, is_active: true }],
  subcategories: [{ id: "s1", category_id: "c1", name: "Birch", sort_order: 0, is_active: true }],
  varieties: [{ id: "v1", subcategory_id: "s1", name: "3mm", default_rate: 90, unit: "sq ft", default_wastage_pct: 10, notes: null, sort_order: 0, is_active: true }],
} satisfies CostRows;

const product = (code: string, cost_price: number, default_sp: number, markup_pct = 0) =>
  ({
    code, name: `Item ${code} & <more>`, source: null, cost_price, default_sp, markup_pct,
    target_margin: 0, is_active: true, categories: { name: "Utility", counts_as_item: true },
  }) as ProductWithCategory;

test("product sheet: SP formula where it gives today's price, typed value otherwise", () => {
  const bytes = buildProductSheet([
    product("TS/0001/WL", 16.13, 100, 83), // stored markup: 16.13 / 0.17 = 94.88 -> 100
    product("TS/0001/BL", 250, 400), // no markup yet: implied 37.5% gives 400 back; same product, so no gap
    product("TS/0002/WL", 250, 395), // not a multiple of 10: the formula would make it 400
    product("TS/0003/WL", 0, 50), // no cost: nothing to work from
  ]);
  const ws = XLSX.read(bytes).Sheets[SHEET_NAME];
  assert.ok(ws, "tab is named Product Master");

  const grid = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1, raw: false, defval: "", blankrows: true });
  assert.equal(grid[0][5], "Markup %");
  const rows = buildImport({ products: grid }).products;
  assert.equal(rows.length, 4, "gap rows and formula-only blank rows are ignored");
  assert.equal(rows[0].name, "Item TS/0001/WL & <more>");
  assert.deepEqual(rows.map((r) => r.default_sp), [100, 400, 395, 50], "a round trip moves no price");
  assert.equal(rows[0].target_margin, 0.83, "Markup % column carries the stored markup");

  const zip = XLSX.CFB.read(bytes, { type: "buffer" });
  const xml = new TextDecoder().decode(
    zip.FileIndex[zip.FullPaths.findIndex((p: string) => p.endsWith("sheet1.xml"))].content as Uint8Array,
  );
  assert.match(xml, /state="frozen"/);
  assert.match(xml, /<c r="G2" s="3"><f>[^<]*CEILING[^<]*<\/f><v>100<\/v>/);
  assert.match(xml, /<c r="G3" s="3"><f>/, "the other colour follows straight on");
  assert.match(xml, /<row r="4"><c r="E4" s="3" t="str"><f>/, "a gap before the next product, carrying the formulas");
  assert.match(xml, /<c r="G5" s="3"><v>395<\/v>/);
  assert.match(xml, /<c r="G7" s="3"><v>50<\/v>/);
  assert.match(xml, /<c r="G8" s="3" t="str"><f>/, "blank rows carry the SP formula");
  assert.match(xml, /<c r="E8" s="3" t="str"><f>IF\(OR\(\$A8=&quot;&quot;,COUNTIF\('Cost Lines'!/, "and the lines-sum cost formula");
});

test("product sheet: Cost Lines and overheads round-trip without moving the cost", () => {
  const line = (over: Partial<ProductCostLine>): ProductCostLine => ({
    stage_code: "material", category_name: "Woodbased", subcategory_name: "Birch", variety_name: "3mm", item_name: "Birch 3mm",
    cost_variety_id: "v1", length: 24, breadth: 12, dimension_unit: "inch", unit: "sq ft", rate: 90, qty: 1, wastage_pct: 10,
    duration_minutes: null, line_total: 198, mrp: null, sort_order: 1, ...over,
  });
  const lines = [
    line({}),
    line({ stage_code: "machine", category_name: "Laser", subcategory_name: null, variety_name: "Cutting", cost_variety_id: null, length: null, breadth: null, unit: "hour", rate: 120, duration_minutes: 30, wastage_pct: 0, line_total: 60, sort_order: 2 }),
  ];
  // 198 × 1.1 overhead + 60
  const costed = { ...product("E", 277.8, 560, 50), product_cost_sheets: [{ stage_overheads: { material: 10 }, product_cost_lines: lines }] };
  const other = { ...product("G", 12, 30), product_cost_sheets: [{ stage_overheads: null, product_cost_lines: [line({ cost_variety_id: null, line_total: 12, rate: 6 })] }] };
  const wb = XLSX.read(buildProductSheet([costed, product("F", 99, 200), other], RATES));
  assert.deepEqual(wb.SheetNames, [SHEET_NAME, LINES_SHEET, RATES_SHEET]);
  const tab = (name: string) => XLSX.utils.sheet_to_json<string[]>(wb.Sheets[name], { header: 1, raw: false, defval: "", blankrows: true });

  const master = tab(SHEET_NAME);
  assert.deepEqual([master[0][8], master[0][9]], ["Material", "Material OH %"]);
  assert.deepEqual([master[1][8], master[1][14]], ["198.00", "60.00"], "stage columns cache each stage's lines");
  assert.equal(wb.Sheets[SHEET_NAME].E2.f?.startsWith("IF(OR("), true, "the lines give today's cost, so E2 is the formula");
  assert.equal(wb.Sheets[SHEET_NAME].A3, undefined, "a gap row between products");
  assert.equal(wb.Sheets[SHEET_NAME].E4.f, undefined, "a product without lines keeps a typed cost");

  const ws = wb.Sheets[LINES_SHEET];
  assert.equal(ws.B2.v, "Item E & <more>", "lines show the product's name");
  assert.match(ws.B2.f ?? "", /MATCH\(A2,'Product Master'!\$A:\$A/, "looked up from the Product Master tab");
  assert.match(ws.M2.f ?? "", /MATCH\(/, "a master item's rate is looked up");
  assert.equal(ws.M3.f, undefined, "an item not on the Rates page keeps its typed rate");
  assert.equal(ws.A4, undefined, "a gap row before the next product's lines");
  assert.equal(ws.A5.v, "G");
  assert.equal(tab(RATES_SHEET)[1].join(), "Material,Woodbased,Birch,3mm,sq ft,90.00,10,Material|Woodbased|Birch|3mm");

  const result = buildImport({ products: master });
  const warnings: string[] = [];
  const read = readCostLines(tab(LINES_SHEET), buildCostTree(RATES), warnings, []).get("E")!;
  assert.deepEqual(read.map((l) => l.line_total), [198, 60]);
  assert.equal(result.products[0].cost_price, 277.8);
  const saved = { sheetId: "s1", overheads: { material: 10 }, lines };
  assert.equal(planSheetCosting(result.products[0], read, readOverheads(master[1]), saved, warnings), null, "nothing changed");
});
