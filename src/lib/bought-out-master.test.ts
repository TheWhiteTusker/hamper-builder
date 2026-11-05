import { test } from "node:test";
import assert from "node:assert/strict";
import type { SupabaseClient } from "@supabase/supabase-js";
import { linkBoughtOutLines } from "./bought-out-master.ts";
import type { ProductCostLine } from "./types.ts";

type Row = Record<string, unknown>;

/** Just enough of the Supabase query builder for linkBoughtOutLines. */
function fakeDb(tables: Record<string, Row[]>) {
  return {
    from(table: string) {
      const rows = (tables[table] ??= []);
      let match: (r: Row) => boolean = () => true;
      let patch: Row | null = null;
      let inserted: Row | null = null;
      const q = {
        select: () => q,
        eq: (col: string, val: unknown) => ((match = (r) => r[col] === val), q),
        insert: (v: Row) => ((inserted = { id: `${table}-${rows.length + 1}`, is_active: true, ...v }), rows.push(inserted), q),
        update: (v: Row) => ((patch = v), q),
        maybeSingle: async () => ({ data: rows.find(match) ?? null, error: null }),
        single: async () => ({ data: inserted, error: null }),
        then: (done: (r: unknown) => void) => {
          if (patch) rows.filter(match).forEach((r) => Object.assign(r, patch));
          done({ data: rows.filter(match), error: null });
        },
      };
      return q;
    },
  } as unknown as SupabaseClient;
}

const line = (p: Partial<ProductCostLine>): ProductCostLine => ({
  stage_code: "bought_out", category_name: "", item_name: "", unit: "piece", rate: 0, qty: 1, wastage_pct: 0, line_total: 0, ...p,
});

test("bought-out lines are found or created in the master and linked", async () => {
  const tables: Record<string, Row[]> = {
    cost_stages: [{ id: "bo", code: "bought_out" }],
    cost_categories: [{ id: "c1", stage_id: "bo", name: "Drinkware", is_active: true }],
    cost_subcategories: [{ id: "s1", category_id: "c1", name: "Steel Bottle", is_active: false }],
    cost_varieties: [{ id: "v1", subcategory_id: "s1", name: "500ml", default_rate: 300, unit: "piece", is_active: true }],
  };
  const res = await linkBoughtOutLines(fakeDb(tables), [
    line({ stage_code: "material", category_name: "Woodbased" }),
    // Existing names in another case, new rate: linked, rate updated, hidden subcategory shown again.
    line({ category_name: "drinkware", subcategory_name: "steel bottle", variety_name: "500ML", rate: 320 }),
    // All new: one of each level created.
    line({ category_name: "Sweets", subcategory_name: "Chocolate Box", variety_name: "250g", rate: 450, unit: "box" }),
  ]);
  if (res.error !== undefined) assert.fail(res.error);
  assert.equal(res.lines[0].stage_code, "material");
  assert.deepEqual(
    [res.lines[1].category_name, res.lines[1].item_name, res.lines[1].cost_variety_id],
    ["Drinkware", "Steel Bottle 500ml", "v1"],
  );
  assert.deepEqual(res.repriced, ["v1"]);
  assert.equal(tables.cost_varieties[0].default_rate, 320);
  assert.equal(tables.cost_subcategories[0].is_active, true);
  assert.equal(tables.cost_categories.length, 2);
  assert.equal(tables.cost_varieties[1].default_rate, 450);
  assert.equal(tables.cost_varieties[1].unit, "box");
  assert.equal(res.lines[2].cost_variety_id, tables.cost_varieties[1].id);
});

test("a bought-out line missing a level is refused before anything is written", async () => {
  const tables: Record<string, Row[]> = { cost_stages: [{ id: "bo", code: "bought_out" }] };
  const res = await linkBoughtOutLines(fakeDb(tables), [line({ category_name: "Sweets", item_name: "Chocolate" })]);
  assert.match(res.error ?? "", /Fill Category, Subcategory and Variety/);
  assert.equal(tables.cost_categories, undefined);
});
