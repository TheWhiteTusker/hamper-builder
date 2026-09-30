import { test } from "node:test";
import assert from "node:assert/strict";
import { planReprice } from "./reprice-plan.ts";
import type { ProductCostLine, ProductCostSheet } from "./types";

test("planReprice re-rates lines, keeps margin, and prices uncosted colour siblings", () => {
  const line = {
    id: "l1", sheet_id: "s1", stage_code: "misc", category_name: "C", item_name: "Old", unit: "pcs",
    qty: 2, wastage_pct: 0, rate: 10, cost_variety_id: "v1", line_total: 20,
  } as unknown as ProductCostLine;
  const untouched = { ...line, id: "l2", cost_variety_id: "v9", qty: 2, rate: 5, line_total: 10 } as ProductCostLine;
  const sheet = {
    id: "s1", product_id: "p1", product_code: "LC/0001/WL", product_name: "Box", markup_pct: 0, stage_overheads: {},
  } as unknown as ProductCostSheet;

  const plan = planReprice({
    rates: [{ id: "v1", name: "New", default_rate: 20, unit: "pcs", subcategory_name: "Sub" }],
    lines: [line, untouched],
    sheets: [sheet],
    products: [
      { id: "p1", code: "LC/0001/WL", name: "Box", cost_price: 30, default_sp: 60 },
      { id: "p2", code: "LC/0001/BL", name: "Box", cost_price: 30, default_sp: 60 },
      { id: "p3", code: "LC/0001/NT", name: "Box", cost_price: 30, default_sp: 60 },
      { id: "p4", code: "LC/0002/WL", name: "Other", cost_price: 30, default_sp: 60 },
    ],
    costedAlone: new Set(["p1", "p3"]),
  });

  assert.equal(plan.lines.length, 1);
  assert.equal(plan.lines[0].rate, 20);
  assert.equal(plan.lines[0].item_name, "Sub New");
  // p1 is the sheet's own product, p2 an uncosted sibling; p3 has its own sheet, p4 is unrelated.
  assert.deepEqual(plan.products.map((p) => p.id).sort(), ["p1", "p2"]);
  const p1 = plan.products.find((p) => p.id === "p1")!;
  assert.equal(p1.cost_price, 50); // 2 x 20 + 10
  assert.equal(p1.default_sp, 100); // 60 x 50/30, margin kept
  assert.equal(plan.sheets[0].calculated_sp, 100);
});
