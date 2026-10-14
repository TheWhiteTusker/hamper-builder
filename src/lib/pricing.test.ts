import { test } from "node:test";
import assert from "node:assert/strict";
import {
  priceHamper,
  priceQuote,
  priceQuoteLine,
  normalizePct,
  round2,
  spAtMargin,
  marginPctOf,
} from "./pricing.ts";

// A hamper worked by hand, the way it would be on the sheet:
//
//   Dry Fruits   2 x 250 CP / 400 SP     CP 500    SP  800   counts
//   Chocolate    3 x 120 CP / 200 SP     CP 360    SP  600   counts
//   Inside Box   1 x  90 CP / 150 SP     CP  90    SP  150   does NOT count
//                                       -------   ------
//                                        CP 950    SP 1550
//
//   discount 10%   -> after discount 1395
//   target SP 1300 -> variance          +95
//   final SP 1500  -> gross profit      550,  margin 36.67%
//   items counted  -> 2 + 3 = 5  (the box is cost, not an advertised item)
const LINES = [
  { qty: 2, unitCp: 250, unitSp: 400 },
  { qty: 3, unitCp: 120, unitSp: 200 },
  { qty: 1, unitCp: 90, unitSp: 150, countsAsItem: false },
];

test("hamper totals sum the line snapshots", () => {
  const p = priceHamper(LINES);
  assert.equal(p.totalCp, 950);
  assert.equal(p.baseSp, 1550);
});

test("number of items skips categories flagged as not counting", () => {
  assert.equal(priceHamper(LINES).numberOfItems, 5);
  // categoryCountsAsItem() defaults to true for unknown categories
  assert.equal(priceHamper([{ qty: 4, unitCp: 1, unitSp: 2 }]).numberOfItems, 4);
});

test("discount, variance, profit and margin", () => {
  const p = priceHamper(LINES, {
    discountPct: 0.1,
    targetSp: 1300,
    finalCatalogueSp: 1500,
  });
  assert.equal(round2(p.spAfterDiscount), 1395);
  assert.equal(round2(p.variance!), 95);
  assert.equal(round2(p.grossProfit!), 550);
  assert.equal(round2(p.finalMargin! * 100), 36.67);
});

test("profit and margin stay null until a final price is approved", () => {
  const p = priceHamper(LINES, { discountPct: 0.1 });
  assert.equal(p.grossProfit, null);
  assert.equal(p.finalMargin, null);
  assert.equal(p.variance, null);
  // a zero final price must not divide by zero
  assert.equal(priceHamper(LINES, { finalCatalogueSp: 0 }).finalMargin, null);
});

test("gross profit is measured against the approved price, not the computed one", () => {
  // base SP 1550, but the approved catalogue price is 1200
  const p = priceHamper(LINES, { discountPct: 0.5, finalCatalogueSp: 1200 });
  assert.equal(p.grossProfit, 250); // 1200 - 950, discount is irrelevant here
});

test("quote line rate and amount", () => {
  const { finalRate, amount } = priceQuoteLine({
    qty: 10,
    cataloguePrice: 1500,
    discountPct: 0.05,
  });
  assert.equal(finalRate, 1425);
  assert.equal(amount, 14250);
});

test("a blank line discount is treated as zero", () => {
  assert.equal(priceQuoteLine({ qty: 2, cataloguePrice: 100 }).amount, 200);
});

test("combined order rolls up to a grand total", () => {
  const q = priceQuote(
    [
      { qty: 10, cataloguePrice: 1500, discountPct: 0.05 }, // 14250
      { qty: 5, cataloguePrice: 2000 }, // 10000
    ],
    {
      quoteStructure: "Combined Order",
      orderDiscount: 250,
      adj1: 500, // freight
      adj2: 0,
      gstRate: 0.18,
    },
  );
  assert.equal(q.subtotal, 24250);
  assert.equal(q.totalQty, 15);
  assert.equal(q.taxableValue, 24500); // 24250 - 250 + 500
  assert.equal(q.gstAmount, 4410);
  assert.equal(q.grandTotal, 28910);
});

test("option-style quotes have no order total", () => {
  const q = priceQuote([{ qty: 1, cataloguePrice: 1000 }], {
    quoteStructure: "Option Based",
    gstRate: 0.18,
  });
  assert.equal(q.subtotal, 1000); // lines still price individually
  assert.equal(q.taxableValue, null);
  assert.equal(q.grandTotal, null);
});

test("percentages survive both spreadsheet export shapes", () => {
  assert.equal(normalizePct(0.35), 0.35); // percent-formatted cell
  assert.equal(normalizePct(35), 0.35); // exported as a whole number
  assert.equal(normalizePct("80.00%"), 0.8); // .xlsx / Sheets export of a percent cell
  assert.equal(normalizePct(""), 0);
  assert.equal(normalizePct(null), 0);
});

test("hamper line margin and price follow each other", () => {
  // 100 cost at 35% -> 153.85, rounded up to 160, as the calculator prices it.
  assert.equal(spAtMargin(100, 35), 160);
  assert.equal(spAtMargin(100, 100), null); // no finite price
  assert.equal(spAtMargin(0, 35), null); // nothing to price from
  assert.equal(marginPctOf(100, 160), 37.5);
  assert.equal(marginPctOf(100, 0), null);
});
