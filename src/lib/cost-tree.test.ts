import { test } from "node:test";
import assert from "node:assert/strict";
import { applyCostChanges, buildCostTree, type CostRows } from "./cost-tree.ts";

const stage = { id: "s1", code: "material", name: "Material", calculation_type: "dimension" as const, sort_order: 0, is_active: true };
const cat = (id: string, sort_order = 0) => ({ id, stage_id: "s1", name: id, sort_order, is_active: true });
const sub = (id: string, category_id: string, sort_order = 0) => ({ id, category_id, name: id, sort_order, is_active: true });
const variety = (id: string, subcategory_id: string, default_rate = 10) => ({
  id, subcategory_id, name: id, default_rate, unit: "sq ft", default_wastage_pct: 0, notes: null, sort_order: 0, is_active: true,
});

const rows: CostRows = {
  stages: [stage],
  categories: [cat("wood", 1), cat("metal", 2)],
  subcategories: [sub("birch", "wood"), sub("steel", "metal")],
  varieties: [variety("8mm", "birch"), variety("2mm", "steel")],
};

const names = (r: CostRows) =>
  buildCostTree(r)[0].categories.map((c) => `${c.id}: ${c.subcategories.map((s) => `${s.id}[${s.varieties.map((v) => v.id)}]`)}`);

test("buildCostTree nests by parent id in sort_order", () => {
  assert.deepEqual(names(rows), ["wood: birch[8mm]", "metal: steel[2mm]"]);
  const flipped = { ...rows, categories: [cat("wood", 3), cat("metal", 2)] };
  assert.deepEqual(names(flipped), ["metal: steel[2mm]", "wood: birch[8mm]"]);
});

test("upsert replaces a row in place and leaves the others alone", () => {
  const next = applyCostChanges(rows, [{ table: "varieties", upsert: [variety("8mm", "birch", 99)] }]);
  assert.equal(next.varieties[0].default_rate, 99);
  assert.equal(next.varieties[1], rows.varieties[1]);
  assert.equal(next.categories, rows.categories);
  assert.equal(rows.varieties[0].default_rate, 10, "input not mutated");
});

test("upsert of a new id appends it", () => {
  const next = applyCostChanges(rows, [{ table: "categories", upsert: [cat("glass", 2)] }]);
  assert.deepEqual(names(next), ["wood: birch[8mm]", "metal: steel[2mm]", "glass: "]);
});

test("remove drops the row and, through the tree, everything under it", () => {
  const next = applyCostChanges(rows, [{ table: "categories", remove: ["wood"] }]);
  assert.deepEqual(names(next), ["metal: steel[2mm]"]);
});

test("moving a subcategory is an upsert with the new category_id", () => {
  const next = applyCostChanges(rows, [{ table: "subcategories", upsert: [sub("birch", "metal", 1)] }]);
  assert.deepEqual(names(next), ["wood: ", "metal: steel[2mm],birch[8mm]"]);
});

test("duplicating adds the new subcategory and its varieties together", () => {
  const next = applyCostChanges(rows, [
    { table: "subcategories", upsert: [sub("birch-copy", "wood", 1)] },
    { table: "varieties", upsert: [variety("8mm-copy", "birch-copy")] },
  ]);
  assert.deepEqual(names(next), ["wood: birch[8mm],birch-copy[8mm-copy]", "metal: steel[2mm]"]);
});
