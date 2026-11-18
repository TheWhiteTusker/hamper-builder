import { test } from "node:test";
import assert from "node:assert/strict";
import { buildCostTree, type CostRows } from "./cost-tree.ts";
import { LINE_HEADERS, planSheetCosting, readCostLines, readOverheads } from "./product-sheet-costing.ts";

const RATES = {
  stages: [
    { id: "st1", code: "material", name: "Material", calculation_type: "dimension", sort_order: 1, is_active: true },
    { id: "st2", code: "machine", name: "Machine Cost", calculation_type: "time_based", sort_order: 2, is_active: true },
  ],
  categories: [{ id: "c1", stage_id: "st1", name: "Woodbased", sort_order: 0, is_active: true }],
  subcategories: [{ id: "s1", category_id: "c1", name: "Birch", sort_order: 0, is_active: true }],
  varieties: [{ id: "v1", subcategory_id: "s1", name: "3mm", default_rate: 90, unit: "sq ft", default_wastage_pct: 10, notes: null, sort_order: 0, is_active: true }],
} satisfies CostRows;

const row = (cells: Partial<Record<(typeof LINE_HEADERS)[number], string>>) => LINE_HEADERS.map((h) => cells[h] ?? "");
const grid = (...rows: string[][]) => [[...LINE_HEADERS], ...rows];

test("readOverheads: blank and 0 left out; percents read either way", () => {
  const r = Array(20).fill("");
  [r[9], r[13], r[15]] = ["12.50%", "0.05", "0"];
  assert.deepEqual(readOverheads(r), { material: 12.5, finishing: 5 });
});

test("readCostLines: master items take the master's rate; others keep what was typed", () => {
  const warnings: string[] = [];
  const errors: string[] = [];
  const lines = readCostLines(
    grid(
      // Typed rate, unit and wastage are ignored for an item on the Rates page; names any case.
      row({ "Product Code": "A", Type: "material", Category: "woodbased", Subcategory: "BIRCH", Variety: "3mm", Length: "12", Breadth: "12", Rate: "1", Unit: "piece" }),
      row({ "Product Code": "A", Type: "Machine Cost", Category: "Laser", Variety: "Cutting", Minutes: "30", Unit: "hour", Rate: "120" }),
      row({ "Product Code": "B", Type: "Material", Category: "Woodbased", Subcategory: "Birch", Variety: "3mm", Qty: "0" }),
      row({ "Product Code": "B" }), // a code with nothing else: an unused row
    ),
    buildCostTree(RATES),
    warnings,
    errors,
  );
  assert.deepEqual(errors, []);
  const [birch, laser] = lines.get("A")!;
  assert.equal(birch.cost_variety_id, "v1");
  assert.deepEqual([birch.category_name, birch.subcategory_name, birch.unit, birch.rate, birch.wastage_pct], ["Woodbased", "Birch", "sq ft", 90, 10]);
  assert.equal(birch.line_total, 99, "1 sq ft × 90 + 10%");
  assert.deepEqual([laser.stage_code, laser.cost_variety_id, laser.unit, laser.line_total, laser.sort_order], ["machine", null, "hour", 60, 2]);
  assert.equal(lines.get("B")!.length, 1);
  assert.equal(lines.get("B")![0].line_total, 0, "qty 0 costs nothing");
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /^A: Machine Cost › Laser › Cutting isn't on the Rates page, so its typed rate ₹120 is used/);

  readCostLines(grid(row({ "Product Code": "C", Type: "Matrial", Category: "Woodbased" })), buildCostTree(RATES), [], errors);
  assert.match(errors[0], /row 2: Type "Matrial" isn't one of Material, Hardware/);
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
