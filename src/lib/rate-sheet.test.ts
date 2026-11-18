import { test } from "node:test";
import assert from "node:assert/strict";
import * as XLSX from "xlsx";
import type { CostRows } from "./cost-tree.ts";
import { planRateImport } from "./rate-import.ts";
import { buildRateSheet, rateRows, RATE_SHEET_NAME } from "./rate-sheet.ts";

const db: CostRows = {
  stages: [{ id: "s1", code: "material", name: "Material", calculation_type: "dimension", sort_order: 1, is_active: true }],
  categories: [{ id: "c1", stage_id: "s1", name: "Woodbased", sort_order: 0, is_active: true }],
  subcategories: [{ id: "u1", category_id: "c1", name: "Birch", sort_order: 0, is_active: true }],
  varieties: [
    { id: "v1", subcategory_id: "u1", name: "8mm", default_rate: 100, unit: "sq ft", default_wastage_pct: 10, notes: "keep", sort_order: 0, is_active: true },
    { id: "v2", subcategory_id: "u1", name: "12mm", default_rate: 150, unit: "sq ft", default_wastage_pct: 10, notes: null, sort_order: 1, is_active: true },
  ],
};

/** Through a real .xlsx and back, as the browser does it. */
function roundTrip(extra: (string | number)[][] = []) {
  const wb = XLSX.read(buildRateSheet([...rateRows(db), ...extra]));
  return XLSX.utils.sheet_to_json<string[]>(wb.Sheets[RATE_SHEET_NAME], { header: 1, raw: false, defval: "", blankrows: true });
}

test("an untouched export changes nothing", () => {
  const plan = planRateImport(roundTrip(), db, false);
  assert.deepEqual(plan.counts, { newCategories: 0, newSubcategories: 0, newVarieties: 0, updatedVarieties: 0 });
  assert.deepEqual(plan.warnings, []);
});

test("edits update by name path, blanks keep values, new rows build the hierarchy", () => {
  const grid = roundTrip([["material", "Metal", "Steel", "2mm", "80", "", "", ""]]);
  grid[1][4] = "1,250.00"; // 8mm rate
  grid[1][7] = ""; // blank notes keep "keep"
  grid[2][3] = "12MM"; // case-only difference still matches
  const plan = planRateImport(grid, db, false);

  assert.equal(plan.updates.length, 1);
  assert.equal(plan.updates[0].default_rate, 1250);
  assert.equal(plan.updates[0].notes, "keep");
  assert.deepEqual(plan.reprice, ["v1"]);
  assert.deepEqual([...plan.newCategories.values()], [{ stage_id: "s1", name: "Metal", sort_order: 1 }]);
  assert.equal(plan.newSubcategories.size, 1);
  assert.equal(plan.newVarieties[0].unit, "sq ft");
  assert.equal(plan.newVarieties[0].default_wastage_pct, 10);
});

test("add mode skips existing varieties; unknown types are reported", () => {
  const grid = roundTrip([["Plastic", "X", "Y", "Z", "1", "", "", ""]]);
  grid[1][4] = "999";
  const plan = planRateImport(grid, db, true);
  assert.equal(plan.updates.length, 0);
  assert.equal(plan.newVarieties.length, 0);
  assert.equal(plan.warnings.length, 2);
});

test("a blank row comes before each new subcategory, and row numbers in warnings stay right", () => {
  const two: CostRows = {
    ...db,
    subcategories: [...db.subcategories, { id: "u2", category_id: "c1", name: "Pine", sort_order: 1, is_active: true }],
    varieties: [...db.varieties, { id: "v3", subcategory_id: "u2", name: "6mm", default_rate: 60, unit: "sq ft", default_wastage_pct: 5, notes: null, sort_order: 0, is_active: true }],
  };
  const ws = XLSX.read(buildRateSheet([...rateRows(two), ["Material", "", "", "", "", "", "", ""]])).Sheets[RATE_SHEET_NAME];
  const grid = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1, raw: false, defval: "", blankrows: true });
  assert.deepEqual(grid.slice(1, 5).map((r) => r[3]), ["8mm", "12mm", "", "6mm"], "Birch rows together, a gap, then Pine");
  assert.equal(ws.I4.f, 'IF(E4="","",E4*(1+N(G4)/100))', "the gap row keeps the formula");
  assert.equal(ws.I5.f, 'IF(E5="","",E5*(1+N(G5)/100))');
  const plan = planRateImport(grid, two, false);
  assert.deepEqual(plan.warnings, ["Rows missing a Type, Category, Subcategory or Variety were skipped: row 6"]);
  assert.equal(plan.updates.length + plan.newVarieties.length, 0);
});
