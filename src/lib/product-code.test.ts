import { test } from "node:test";
import assert from "node:assert/strict";
import {
  formatProductCode,
  parseProductCode,
  getNextSerialForCategory,
  deriveCategoryCode,
  STANDARD_PRODUCT_COLORS,
  codeClash,
  codesForColors,
  colorCode,
  resolveColors,
} from "./product-code.ts";

test("STANDARD_PRODUCT_COLORS contains exactly Walnut (WL), Natural (NT), and Black (BL)", () => {
  assert.equal(STANDARD_PRODUCT_COLORS.length, 3);
  assert.deepEqual(
    STANDARD_PRODUCT_COLORS.map((c) => ({ name: c.name, code: c.code })),
    [
      { name: "Walnut", code: "WL" },
      { name: "Natural", code: "NT" },
      { name: "Black", code: "BL" },
    ],
  );
});

test("formatProductCode formats correctly into CATEGORY/SERIAL/COLOR", () => {
  assert.equal(formatProductCode("LC", 1, "WL"), "LC/0001/WL");
  assert.equal(formatProductCode("lc", "1", "natural"), "LC/0001/NT");
  assert.equal(formatProductCode("ED", 42, "black"), "ED/0042/BL");
  assert.equal(formatProductCode("DC", "0123", "BL"), "DC/0123/BL");
});

test("parseProductCode parses valid standard product codes", () => {
  const p1 = parseProductCode("LC/0001/WL");
  assert.equal(p1.isValid, true);
  assert.equal(p1.categoryCode, "LC");
  assert.equal(p1.serial, "0001");
  assert.equal(p1.colorCode, "WL");
  assert.equal(p1.colorName, "Walnut");

  const p2 = parseProductCode("ED/0025/NT");
  assert.equal(p2.isValid, true);
  assert.equal(p2.categoryCode, "ED");
  assert.equal(p2.serial, "0025");
  assert.equal(p2.colorCode, "NT");
  assert.equal(p2.colorName, "Natural");

  const p3 = parseProductCode("DC/0099/BL");
  assert.equal(p3.isValid, true);
  assert.equal(p3.categoryCode, "DC");
  assert.equal(p3.serial, "0099");
  assert.equal(p3.colorCode, "BL");
  assert.equal(p3.colorName, "Black");
});

test("parseProductCode handles legacy codes without throwing", () => {
  const p = parseProductCode("LC010");
  assert.equal(p.isValid, false);
  assert.equal(p.categoryCode, "LC");
  assert.equal(p.serial, "0010");
});

test("getNextSerialForCategory computes next 4-digit serial", () => {
  const existing = ["LC001", "LC002", "LC010", "LC/0001/WL", "ED001"];
  const nextLC = getNextSerialForCategory("LC", existing);
  assert.equal(nextLC, "0011");

  const nextED = getNextSerialForCategory("ED", existing);
  assert.equal(nextED, "0002");

  const nextDC = getNextSerialForCategory("DC", existing);
  assert.equal(nextDC, "0001");
});

test("deriveCategoryCode generates 2-letter uppercase initials", () => {
  assert.equal(deriveCategoryCode("Lights & Candles"), "LC");
  assert.equal(deriveCategoryCode("Box & Packaging"), "BP");
  assert.equal(deriveCategoryCode("Decor"), "DE");
  assert.equal(deriveCategoryCode("Utility"), "UT");
});

test("colorCode keeps standard codes and derives codes for added colors", () => {
  assert.equal(colorCode("Walnut"), "WL");
  assert.equal(colorCode("NT"), "NT");
  assert.equal(colorCode("Brown Paper"), "BP");
  assert.equal(colorCode("Teak"), "TE");
  assert.equal(colorCode("Blue"), "BE"); // BL is Black's, so first + last letter
  assert.equal(formatProductCode("LC", 1, "Brown Paper"), "LC/0001/BP");
  assert.equal(parseProductCode("LC/0001/BP").isValid, true);
  // Picked swatch wins, then the standard swatch, then neutral grey
  assert.deepEqual(resolveColors(["Black", "Teak", "White"], { Teak: "#b5651d" }), [
    { name: "Black", code: "BL", hex: "#222222" },
    { name: "Teak", code: "TE", hex: "#b5651d" },
    { name: "White", code: "WH", hex: "#94a3b8" },
  ]);
});

test("colour codes are unique across the live palette", () => {
  const live = ["Walnut", "Natural", "Black", "Brown Paper", "Tan", "White Paper", "Gold", "Jute Brown",
    "Frosted Candle", "Neutral", "Tan Leather", "Brown Leather", "Black Leather", "Green Leather"];
  assert.equal(codeClash(live), null);
  assert.equal(colorCode("Black Leather"), "BR"); // existing products use BR
  assert.deepEqual(codeClash(["Brown Leather", "Brown Linen"]), { name: "Brown Linen", code: "BN", with: "Brown Leather" });
});

test("adding Brown Leather to a Black Leather product makes a separate BN code", () => {
  assert.deepEqual(codesForColors("AC/0002/BR", ["Black Leather", "Brown Leather"]), [
    { color: "Black Leather", code: "AC/0002/BR" },
    { color: "Brown Leather", code: "AC/0002/BN" },
  ]);
});
