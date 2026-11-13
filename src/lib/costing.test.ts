import { test } from "node:test";
import assert from "node:assert/strict";
import {
  calculateDimensionArea,
  calculateLineCost,
  isSizedUnit,
} from "./costing.ts";

test("calculateDimensionArea handles inches to sq ft", () => {
  assert.equal(calculateDimensionArea(12, 12, "inch", "sq ft"), 1);
  assert.equal(calculateDimensionArea(24, 12, "inch", "sq ft"), 2);
  assert.equal(calculateDimensionArea(6, 6, "inch", "sq ft"), 0.25);
});

test("calculateDimensionArea handles mm to sq ft", () => {
  const area = calculateDimensionArea(304.8, 304.8, "mm", "sq ft");
  assert.equal(Math.round(area), 1);
});

test("calculateDimensionArea handles feet (linear)", () => {
  assert.equal(calculateDimensionArea(24, 0, "inch", "feet"), 2);
  assert.equal(calculateDimensionArea(36, 12, "inch", "feet"), 3);
  assert.equal(calculateDimensionArea(304.8, 0, "mm", "feet"), 1);
  assert.equal(calculateDimensionArea(30.48, 0, "cm", "feet"), 1);
});

test("calculateDimensionArea handles inch, mm, and cm (linear)", () => {
  assert.equal(calculateDimensionArea(12, 0, "inch", "inch"), 12);
  assert.equal(calculateDimensionArea(50.8, 0, "mm", "inch"), 2);
  assert.equal(calculateDimensionArea(500, 0, "mm", "mm"), 500);
  assert.equal(calculateDimensionArea(50, 0, "cm", "mm"), 500);
  assert.equal(calculateDimensionArea(2, 0, "inch", "mm"), 50.8);
  assert.equal(calculateDimensionArea(25, 0, "cm", "cm"), 25);
  assert.equal(calculateDimensionArea(100, 0, "mm", "cm"), 10);
  assert.equal(calculateDimensionArea(2, 0, "inch", "cm"), 5.08);
});

test("calculateDimensionArea handles sq inch", () => {
  assert.equal(calculateDimensionArea(10, 5, "inch", "sq inch"), 50);
  assert.equal(calculateDimensionArea(25.4, 25.4, "mm", "sq inch"), 1);
});

test("calculateDimensionArea handles sq mm", () => {
  assert.equal(calculateDimensionArea(100, 200, "mm", "sq mm"), 20000);
  assert.equal(calculateDimensionArea(10, 20, "cm", "sq mm"), 20000);
});

test("calculateDimensionArea handles sq cm", () => {
  assert.equal(calculateDimensionArea(10, 20, "cm", "sq cm"), 200);
  assert.equal(calculateDimensionArea(100, 200, "mm", "sq cm"), 200);
  assert.equal(calculateDimensionArea(2, 4, "inch", "sq cm"), 51.61);
});

test("calculateLineCost for dimensional material", () => {
  const line = calculateLineCost({
    stage_code: "material",
    length: 12,
    breadth: 12,
    dimension_unit: "inch",
    unit: "sq ft",
    rate: 85,
    qty: 2,
    wastage_pct: 10,
  });

  assert.equal(line.calculated_area, 1);
  assert.equal(line.line_total, 187);
});

test("calculateLineCost for machine per minute cost", () => {
  const line = calculateLineCost({
    stage_code: "machine",
    duration_minutes: 15,
    rate: 12,
    qty: 1,
    wastage_pct: 5,
  });

  assert.equal(line.calculated_area, 15);
  assert.equal(line.line_total, 189);
});

test("calculateLineCost for linear feet material", () => {
  const line = calculateLineCost({
    stage_code: "material",
    length: 36,
    dimension_unit: "inch",
    unit: "feet",
    rate: 20,
    qty: 2,
    wastage_pct: 0,
  });

  assert.equal(line.calculated_area, 3);
  assert.equal(line.line_total, 120);
});

test("calculateLineCost for sq cm finishing", () => {
  const line = calculateLineCost({
    stage_code: "finishing",
    length: 10,
    breadth: 20,
    dimension_unit: "cm",
    unit: "sq cm",
    rate: 0.5,
    qty: 1,
    wastage_pct: 0,
  });

  assert.equal(line.calculated_area, 200);
  assert.equal(line.line_total, 100);
});

test("isSizedUnit: dimensions count for length/area rates, not per piece", () => {
  for (const u of ["sqft", "sq inch", "sqinch", "meter", "running ft", "", null]) assert.equal(isSizedUnit(u), true, String(u));
  for (const u of ["piece", "set", "kg", "min"]) assert.equal(isSizedUnit(u), false, u);
  assert.equal(calculateLineCost({ stage_code: "material", unit: "sqinch", rate: 2, qty: 1, length: 3, breadth: 4 }).line_total, 24);
});
