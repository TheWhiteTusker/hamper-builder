import { test } from "node:test";
import assert from "node:assert/strict";
import { mergeHamper, mergeQuote } from "./live-merge.ts";
import type { HamperSummary, QuoteSummary } from "./types-docs";

const hamper = {
  id: "h1", code: "H-1", name: "Gold", collection: null, status: "Draft",
  number_of_items: 4, total_cp: 800, base_sp: 1100, discount_pct: 0, sp_after_discount: 1100,
  target_sp: null, variance: null, final_catalogue_sp: 1000, gross_profit: 200, final_margin: 0.2,
  notes: null, created_by: null, created_at: "", updated_at: "", line_count: 4, image_url: null,
} satisfies HamperSummary;

test("mergeHamper recomputes profit and margin from the new price, keeps item totals", () => {
  const next = mergeHamper(hamper, { id: "h1", name: "Gold+", final_catalogue_sp: 1200 });
  assert.equal(next.name, "Gold+");
  assert.equal(next.gross_profit, 400);
  assert.ok(Math.abs((next.final_margin ?? 0) - 1 / 3) < 1e-9);
  assert.equal(next.total_cp, 800);
  assert.equal(next.number_of_items, 4);
});

test("mergeHamper: no catalogue price means no profit or margin", () => {
  const next = mergeHamper(hamper, { id: "h1", final_catalogue_sp: null });
  assert.equal(next.gross_profit, null);
  assert.equal(next.final_margin, null);
});

test("mergeHamper never copies the canvas into the list row", () => {
  const next = mergeHamper(hamper, { id: "h1", canvas: { layers: [] } });
  assert.equal("canvas" in next, false);
});

test("mergeHamper accepts numbers sent as strings", () => {
  const next = mergeHamper(hamper, { id: "h1", final_catalogue_sp: "900.50" });
  assert.equal(next.gross_profit, 100.5);
});

const quote = {
  id: "q1", doc_no: "Q-1", doc_type: "quotation", doc_date: "2026-11-23", client_name: "Acme",
  contact_person: null, phone: null, email: null, billing_address: null, gstin: null, occasion: null,
  quote_structure: "Combined Order", validity: null, status: "Sent", order_discount: 500, adj1: 0, adj2: 0,
  gst_rate: 0.18, notes: null, terms: null, follow_up_date: null, linked_doc_no: null, created_by: null,
  created_at: "", updated_at: "", subtotal: 10000, total_qty: 10, line_count: 2,
  taxable_value: 9500, gst_amount: 1710, grand_total: 11210,
} satisfies QuoteSummary;

test("mergeQuote recomputes the total from the new GST rate", () => {
  const next = mergeQuote(quote, { id: "q1", gst_rate: 0.12, status: "Won" });
  assert.equal(next.status, "Won");
  assert.equal(next.taxable_value, 9500);
  assert.ok(Math.abs((next.grand_total ?? 0) - 9500 * 1.12) < 1e-9);
  assert.equal(next.line_count, 2);
});

test("mergeQuote: an option-based quotation has no single total", () => {
  const next = mergeQuote(quote, { id: "q1", quote_structure: "Option Based" });
  assert.equal(next.grand_total, null);
});
