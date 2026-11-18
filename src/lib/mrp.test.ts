import { test } from "node:test";
import assert from "node:assert/strict";
import { mrpLimit, mrpMargin, roundSp } from "./mrp.ts";
import { planReprice } from "./reprice-plan.ts";
import type { ProductCostLine, ProductCostSheet } from "./types";

const piece = (rate: number, mrp?: number) =>
  ({ stage_code: "bought_out", unit: "piece", rate, qty: 1, wastage_pct: 0, mrp }) as ProductCostLine;

test("mrpLimit keeps SP + GST within the MRP", () => {
  assert.equal(mrpLimit([piece(300, 499)], {}, 300, 0.18), 422); // 499 / 1.18 = 422.88
  assert.equal(mrpLimit([{ ...piece(300, 499), qty: 2 }], {}, 600, 0.18), 845); // 998 / 1.18
  assert.equal(mrpLimit([piece(300)], {}, 300, 0.18), null);
  // Exact division is not lost to float error: 1180 / 1.18 = 1000.
  assert.equal(mrpLimit([piece(800, 1180)], {}, 800, 0.18), 1000);
});

test("mrpLimit caps only the MRP items' share of a mixed product", () => {
  const box = { ...piece(200), stage_code: "material" };
  // The jar is 300 of 500 cost (60%), so SP can reach 400 / 0.6.
  assert.equal(mrpLimit([box, piece(300, 400)], {}, 500, 0), 666);
  // A free MRP item takes none of the price.
  assert.equal(mrpLimit([box, piece(0, 400)], {}, 200, 0), null);
  // Nothing costed yet: the MRP alone is the cap.
  assert.equal(mrpLimit([piece(0, 400)], {}, 0, 0), 400);
});

test("roundSp never rounds an SP within the MRP limit over it", () => {
  assert.equal(roundSp(415, 422), 420);
  assert.equal(roundSp(421, 422), 422);
  assert.equal(roundSp(422.003, 422), 422); // markup round-trip drift
  assert.equal(roundSp(423, 422), 430); // already over: left for the alert
  assert.equal(roundSp(421, null), 430);
});

test("mrpMargin is the margin at MRP less GST", () => {
  assert.equal(Math.round(mrpMargin(piece(300, 499), 0.18)! * 100), 29);
  assert.equal(mrpMargin(piece(300), 0.18), null);
});

test("planReprice never scales a price past the MRP limit", () => {
  const line = { ...piece(300, 499), id: "l1", sheet_id: "s1", cost_variety_id: "v1", line_total: 300 } as ProductCostLine;
  const sheet = { id: "s1", product_id: "p1", product_code: "BO/0001/WL", markup_pct: 0, stage_overheads: {} } as unknown as ProductCostSheet;
  const plan = planReprice({
    rates: [{ id: "v1", name: "500ml", default_rate: 330, unit: "piece", default_wastage_pct: 0 }],
    lines: [line],
    sheets: [sheet],
    products: [{ id: "p1", code: "BO/0001/WL", name: "Bottle", cost_price: 300, default_sp: 420 }],
    costedAlone: new Set(["p1"]),
    gstRate: 0.18,
  });
  // 420 x 330/300 = 462 would round to 470; MRP 499 caps it at 422.
  assert.equal(plan.products[0].default_sp, 422);
  assert.equal(plan.sheets[0].calculated_sp, 422);
});
