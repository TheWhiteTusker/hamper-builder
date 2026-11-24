import { test } from "node:test";
import assert from "node:assert/strict";
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { LINE_HEADERS, planSheetCosting, readCostLines, readOverheads } from "./product-sheet-costing.ts";

const RATES = {
  stages: [
    { id: "st1", code: "material", name: "Material", calculation_type: "dimension", sort_order: 1, is_active: true },
    { id: "st2", code: "machine", name: "Machine Cost", calculation_type: "time_based", sort_order: 2, is_active: true },
  ],
  categories: [
    { id: "c1", stage_id: "st1", name: "Woodbased", sort_order: 0, is_active: true },
    { id: "c2", stage_id: "st2", name: "Laser", sort_order: 0, is_active: true },
  ],
  subcategories: [
    { id: "s1", category_id: "c1", name: "Birch", sort_order: 0, is_active: true },
    { id: "s2", category_id: "c2", name: "CO2", sort_order: 0, is_active: true },
  ],
  varieties: [
    { id: "v1", subcategory_id: "s1", name: "3mm", default_rate: 90, unit: "sq ft", default_wastage_pct: 10, notes: null, sort_order: 0, is_active: true },
    { id: "v2", subcategory_id: "s2", name: "Cutting", default_rate: 120, unit: "hour", default_wastage_pct: 0, notes: null, sort_order: 0, is_active: true },
  ],
} satisfies CostRows;

const row = (cells: Partial<Record<(typeof LINE_HEADERS)[number], string>>) => LINE_HEADERS.map((h) => cells[h] ?? "");
const grid = (...rows: string[][]) => [[...LINE_HEADERS], ...rows];

test("readOverheads: blank and 0 left out; percents read either way", () => {
  const r = Array(20).fill("");
  [r[9], r[13], r[15]] = ["12.50%", "0.05", "0"];
  assert.deepEqual(readOverheads(r), { material: 12.5, finishing: 5 });
});

test("readCostLines: items take the Rates page's rate; ones not on it are errors", () => {
  const warnings: string[] = [];
  const errors: string[] = [];
  const lines = readCostLines(
    grid(
      // Typed rate, unit and wastage are ignored for an item on the Rates page; names any case.
      row({ "Product Code": "A", Type: "material", Category: "woodbased", Subcategory: "BIRCH", Variety: "3mm", Length: "12", Breadth: "12", Rate: "1", Unit: "piece" }),
      row({ "Product Code": "A", Type: "Machine Cost", Category: "Laser", Subcategory: "co2", Variety: "Cutting", Minutes: "30", Rate: "5" }),
      row({ "Product Code": "B", Type: "Material", Category: "Woodbased", Subcategory: "Birch", Variety: "3mm", Qty: "0" }),
      row({ "Product Code": "B" }), // a code with nothing else: an unused row
    ),
    buildCostTree(RATES),
    warnings,
    errors,
  );
  assert.deepEqual(errors, []);
  assert.deepEqual(warnings, []);
  const [birch, laser] = lines.get("A")!;
  assert.equal(birch.cost_variety_id, "v1");
  assert.deepEqual([birch.category_name, birch.subcategory_name, birch.unit, birch.rate, birch.wastage_pct], ["Woodbased", "Birch", "sq ft", 90, 10]);
  assert.equal(birch.line_total, 99, "1 sq ft × 90 + 10%");
  assert.deepEqual([laser.stage_code, laser.cost_variety_id, laser.subcategory_name, laser.unit, laser.line_total, laser.sort_order], ["machine", "v2", "CO2", "hour", 60, 2]);
  assert.equal(lines.get("B")!.length, 1);
  assert.equal(lines.get("B")![0].line_total, 0, "qty 0 costs nothing");

  // A near miss ("Birches" for "Birch") is not saved unlinked: it stops the import.
  const missed = readCostLines(
    grid(row({ "Product Code": "D", Type: "Material", Category: "Woodbased", Subcategory: "Birches", Variety: "3mm", Rate: "90" })),
    buildCostTree(RATES),
    warnings,
    errors,
  );
  assert.equal(missed.size, 0);
  assert.match(errors[0], /^Cost Lines row 2: D's Material › Woodbased › Birches › 3mm isn't on the Rates page/);
  errors.length = 0;

  readCostLines(grid(row({ "Product Code": "C", Type: "Matrial", Category: "Woodbased" })), buildCostTree(RATES), [], errors);
  // Only the stages in use are offered: a hidden one (Misc) isn't loaded, so isn't listed.
  assert.match(errors[0], /row 2: Type "Matrial" isn't one of Material, Machine\.$/);
});

test("readCostLines: a typed wastage changes the Rates item for all its lines", () => {
  const birch = { Type: "Material", Category: "Woodbased", Subcategory: "Birch", Variety: "3mm", Length: "12", Breadth: "12" };
  const [warnings, errors, wastage] = [[] as string[], [] as string[], new Map<string, number>()];
  const lines = readCostLines(
    grid(
      row({ "Product Code": "A", ...birch, "Wastage %": "20" }),
      row({ "Product Code": "B", ...birch, "Wastage %": "10" }), // untouched: still the Rates value
    ),
    buildCostTree(RATES),
    warnings,
    errors,
    wastage,
  );
  assert.deepEqual(errors, []);
  assert.deepEqual([...wastage], [["v1", 20]]);
  assert.deepEqual([lines.get("A")![0].line_total, lines.get("B")![0].line_total], [108, 108], "1 sq ft × 90 + 20%, both lines");
  assert.match(warnings[0], /wastage 10% → 20% on the Rates page/);

  readCostLines(grid(row({ "Product Code": "A", ...birch, "Wastage %": "20" }), row({ "Product Code": "B", ...birch, "Wastage %": "15" })), buildCostTree(RATES), [], errors);
  assert.match(errors[0], /row 3: .* has wastage 15% here but 20% on another row/);
});

test("planSheetCosting: unchanged is null; new lines or overheads replace the costing", () => {
  const lines = readCostLines(
    grid(row({ "Product Code": "A", Type: "Material", Category: "Woodbased", Subcategory: "Birch", Variety: "3mm", Length: "24", Breadth: "12" })),
    buildCostTree(RATES),
    [],
    [],
  ).get("A")!;
  const p = { code: "A", cost_price: 217.8, markup_pct: 50 };
  const saved = { sheetId: "s1", overheads: { material: 10 }, lines: lines.map((l) => ({ ...l, id: "x", sheet_id: "s1" })) };
  assert.equal(planSheetCosting(p, lines, { material: 10 }, saved, []), null, "a round trip writes nothing");

  const warnings: string[] = [];
  const plan = planSheetCosting(p, lines, {}, saved, warnings);
  assert.ok(plan, "dropping the overhead is a change");
  assert.equal(plan.sheetId, "s1");
  assert.equal(plan.totals.total_cost, 198, "2 sq ft × 90 + 10% wastage, no overhead");
  assert.match(warnings[0], /lines cost ₹198, not the ₹217.8 in Cost Price/);

  const fresh = planSheetCosting({ ...p, cost_price: 198 }, lines, {}, undefined, warnings);
  assert.equal(fresh?.totals.total_cost, 198);
  assert.equal(fresh?.sheetId, undefined, "no sheet yet: one is created");
  assert.equal(warnings.length, 1, "Cost Price agrees: no warning");
});
