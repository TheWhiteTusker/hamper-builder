// A list row after a live change. The Hampers and Quotes lists read summary
// views, which Realtime can't watch: the change is the base table's row. Copy
// what it carries and recompute what follows from it the way the views do
// (hamper_summary in 0021, quote_summary in 0001). Totals that need the items
// arrive with the background refresh.

import { num } from "./numbers.ts";
import type { HamperSummary, QuoteSummary } from "./types-docs";

type Row = Record<string, unknown>;

// hampers columns the list shows; never `canvas`, which is large.
const HAMPER_FIELDS = [
  "code", "name", "collection", "status", "target_sp", "notes", "discount_pct",
  "final_catalogue_sp", "image_url", "updated_at",
] as const;

export function mergeHamper(cur: HamperSummary, row: Row): HamperSummary {
  const h = { ...cur };
  for (const f of HAMPER_FIELDS) if (f in row) (h as Row)[f] = row[f];
  const sp = h.final_catalogue_sp == null ? null : num(h.final_catalogue_sp);
  const cp = num(h.total_cp);
  h.sp_after_discount = num(h.base_sp) * (1 - num(h.discount_pct));
  h.variance = h.target_sp == null ? null : h.sp_after_discount - num(h.target_sp);
  h.gross_profit = sp == null ? null : sp - cp;
  h.final_margin = !sp ? null : (sp - cp) / sp;
  return h;
}

export function mergeQuote(cur: QuoteSummary, row: Row): QuoteSummary {
  const q = { ...cur, ...row } as QuoteSummary;
  q.taxable_value =
    q.quote_structure === "Combined Order"
      ? num(q.subtotal) - num(q.order_discount) + num(q.adj1) + num(q.adj2)
      : null;
  q.gst_amount = q.taxable_value == null ? null : q.taxable_value * num(q.gst_rate);
  q.grand_total = q.taxable_value == null ? null : q.taxable_value * (1 + num(q.gst_rate));
  return q;
}
