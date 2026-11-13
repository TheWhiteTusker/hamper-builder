import { test } from "node:test";
import assert from "node:assert/strict";
import * as XLSX from "xlsx";
import { buildProductSheet, SHEET_NAME } from "./product-sheet.ts";
import { buildImport } from "./import.ts";
import type { ProductWithCategory } from "./types.ts";

const product = (code: string, cost_price: number, default_sp: number, markup_pct = 0) =>
  ({
    code, name: `Item ${code} & <more>`, source: null, cost_price, default_sp, markup_pct,
    target_margin: 0, is_active: true, categories: { name: "Utility", counts_as_item: true },
  }) as ProductWithCategory;

test("product sheet: SP formula where it gives today's price, typed value otherwise", () => {
  const bytes = buildProductSheet([
    product("A", 16.13, 100, 83), // stored markup: 16.13 / 0.17 = 94.88 -> 100
    product("B", 250, 400), // no markup yet: implied 37.5% gives 400 back
    product("C", 250, 395), // not a multiple of 10: the formula would make it 400
    product("D", 0, 50), // no cost: nothing to work from
  ]);
  const ws = XLSX.read(bytes).Sheets[SHEET_NAME];
  assert.ok(ws, "tab is named Product Master");

  const grid = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1, raw: false, defval: "", blankrows: false });
  assert.equal(grid[0][5], "Markup %");
  const rows = buildImport({ products: grid }).products;
  assert.equal(rows.length, 4, "formula-only blank rows are ignored");
  assert.equal(rows[0].name, "Item A & <more>");
  assert.deepEqual(rows.map((r) => r.default_sp), [100, 400, 395, 50], "a round trip moves no price");
  assert.equal(rows[0].target_margin, 0.83, "Markup % column carries the stored markup");

  const zip = XLSX.CFB.read(bytes, { type: "buffer" });
  const xml = new TextDecoder().decode(
    zip.FileIndex[zip.FullPaths.findIndex((p: string) => p.endsWith("sheet1.xml"))].content as Uint8Array,
  );
  assert.match(xml, /state="frozen"/);
  assert.match(xml, /<c r="G2" s="3"><f>[^<]*CEILING[^<]*<\/f><v>100<\/v>/);
  assert.match(xml, /<c r="G3" s="3"><f>/);
  assert.match(xml, /<c r="G4" s="3"><v>395<\/v>/);
  assert.match(xml, /<c r="G5" s="3"><v>50<\/v>/);
  assert.match(xml, /<c r="G6" s="3" t="str"><f>/, "blank rows carry the SP formula");
});
