import { test } from "node:test";
import assert from "node:assert/strict";
import { filterComboOptions } from "./combo-options.ts";

const current = ["Gold Box", "Silver Box"];
const all = ["Bronze Box", "Gold Box", "Gold Ribbon", "Silver Box"];

test("not typing, or nothing typed: the current group as is", () => {
  assert.equal(filterComboOptions(current, all, "gold", false), current);
  assert.equal(filterComboOptions(current, all, "  ", true), current);
});

test("typing: matches from every option, the current group's first, any case", () => {
  assert.deepEqual(filterComboOptions(current, all, "GOLD", true), ["Gold Box", "Gold Ribbon"]);
  assert.deepEqual(filterComboOptions(current, all, "box", true), ["Gold Box", "Silver Box", "Bronze Box"]);
});

test("typing without allOptions searches the current group", () => {
  assert.deepEqual(filterComboOptions(current, undefined, "silver", true), ["Silver Box"]);
  assert.deepEqual(filterComboOptions(current, [], "zzz", true), []);
});
