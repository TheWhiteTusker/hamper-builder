import { test } from "node:test";
import assert from "node:assert/strict";
import {
  calculateCostSheetTotals,
  calculateLineCost,
  scaledPrice,
  withOverhead,
} from "./costing.ts";

test("calculateCostSheetTotals aggregates all stages and applies markup", () => {
  const lines = [
    {
      stage_code: "material",
      length: 12,
      breadth: 12,
      dimension_unit: "inch",
      unit: "sq ft",
      rate: 85,
      qty: 2,
      wastage_pct: 10,
    },
    {
      stage_code: "hardware",
      unit: "piece",
      rate: 10,
      qty: 4,
      wastage_pct: 0,
    },
    {
      stage_code: "finishing",
      length: 12,
      breadth: 12,
      dimension_unit: "inch",
      unit: "sq ft",
      rate: 35,
      qty: 1,
      wastage_pct: 0,
    },
    {
      stage_code: "machine",
      duration_minutes: 15,
      rate: 12,
      qty: 1,
      wastage_pct: 5,
    },
  ];

  const totals = calculateCostSheetTotals(lines, 50);

  assert.equal(totals.material_total, 187);
  assert.equal(totals.hardware_total, 40);
  assert.equal(totals.finishing_total, 35);
  assert.equal(totals.machine_total, 189);
  assert.equal(totals.total_cost, 451);
  assert.equal(totals.markup_pct, 50);
  assert.equal(totals.calculated_sp, 910);
  assert.equal(totals.target_margin, 0.5);
});

test("miscellaneous and bought-out lines roll into total cost", () => {
  const lines = [
    { stage_code: "material", unit: "piece", rate: 100, qty: 1, wastage_pct: 0 },
    { stage_code: "miscellaneous", unit: "piece", rate: 50, qty: 2, wastage_pct: 0 },
    { stage_code: "bought_out", unit: "piece", rate: 200, qty: 1, wastage_pct: 25 },
    { stage_code: "hardware", unit: "piece", rate: 300, qty: 0, wastage_pct: 0 },
  ];

  const totals = calculateCostSheetTotals(lines, 50);

  assert.equal(totals.material_total, 100);
  assert.equal(totals.hardware_total, 0);
  assert.equal(totals.other_total, 350); // bought-out 200 + 25% wastage
  assert.equal(totals.total_cost, 450);
  assert.equal(totals.calculated_sp, 900);
});

test("SP = CP / (1 - markup%), and 100%+ markup falls back to cost price", () => {
  const lines = [
    { stage_code: "material", unit: "piece", rate: 600, qty: 1, wastage_pct: 0 },
  ];

  const forty = calculateCostSheetTotals(lines, 40);
  assert.equal(forty.total_cost, 600);
  assert.equal(forty.calculated_sp, 1000);
  assert.equal(forty.target_margin, 0.4);

  assert.equal(calculateCostSheetTotals(lines, 0).calculated_sp, 600);

  for (const bad of [100, 150]) {
    const sp = calculateCostSheetTotals(lines, bad).calculated_sp;
    assert.ok(Number.isFinite(sp), `markup ${bad}% produced ${sp}`);
    assert.equal(sp, 600);
  }
});

test("a stage's overhead % is added to that stage's subtotal only", () => {
  const lines = [
    { stage_code: "material", unit: "piece", rate: 100, qty: 2, wastage_pct: 0 },
    { stage_code: "hardware", unit: "piece", rate: 50, qty: 1, wastage_pct: 0 },
    { stage_code: "miscellaneous", unit: "piece", rate: 40, qty: 1, wastage_pct: 0 },
  ];

  const totals = calculateCostSheetTotals(lines, 50, { material: 10, miscellaneous: "25" });

  assert.equal(totals.material_total, 220);
  assert.equal(totals.hardware_total, 50);
  assert.equal(totals.other_total, 50);
  assert.equal(totals.total_cost, 320);
  assert.equal(totals.calculated_sp, 640);

  assert.equal(calculateCostSheetTotals(lines, 50, { material: "" }).total_cost, 290);
  assert.equal(withOverhead(200, 10), 220);
  assert.equal(withOverhead(99.99, ""), 99.99);
});

test("machine rates follow their unit: per minute, per hour, or by size", () => {
  const base = { stage_code: "machine", duration_minutes: 30, qty: 1, wastage_pct: 0 };
  assert.equal(calculateLineCost({ ...base, unit: "min", rate: 3 }).line_total, 90);
  assert.equal(calculateLineCost({ ...base, rate: 3 }).line_total, 90);
  assert.equal(calculateLineCost({ ...base, unit: "hour", rate: 600 }).line_total, 300);
  const engraving = calculateLineCost({ ...base, unit: "sq inch", rate: 0.25, length: 4, breadth: 5, dimension_unit: "inch" });
  assert.equal(engraving.calculated_area, 20);
  assert.equal(engraving.line_total, 5);
});

test("the exact area is costed; the stored area is rounded for display", () => {
  const line = calculateLineCost({ stage_code: "material", unit: "sq ft", rate: 1000, length: 5, breadth: 5, dimension_unit: "inch", qty: 1, wastage_pct: 0 });
  assert.equal(line.calculated_area, 0.17);
  assert.equal(line.line_total, 173.61);
});

test("a repriced product keeps its margin, including a hand-set price", () => {
  assert.equal(scaledPrice(55.5, 277.5, 61.05, 999), 310);
  assert.equal(scaledPrice(321.78, 2883.3, 353.96, 999), 3180);
  assert.equal(scaledPrice(0, 0, 100, 500), 500);
});
