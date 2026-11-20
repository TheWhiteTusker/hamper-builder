import { test } from "node:test";
import assert from "node:assert/strict";
import { listReturn } from "./list-return.ts";

test("listReturn keeps the list's filters and nothing else", () => {
  assert.equal(listReturn("hampers", "/hampers?q=gold&limit=100"), "/hampers?q=gold&limit=100");
  assert.equal(listReturn("hampers", "/hampers?deleted=1&q=x"), "/hampers?q=x");
  assert.equal(listReturn("products", "/products?category=7", true), "/products?category=7&deleted=1");
  for (const bad of ["//evil.com/hampers", "https://evil.com/hampers", "/products?q=1", "javascript:alert(1)", "", null]) {
    assert.equal(listReturn("hampers", bad), "/hampers");
  }
  assert.equal(listReturn("hampers", null, true), "/hampers?deleted=1");
});
