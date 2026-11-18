import { calculateLineCost, type StageOverheads } from "./costing.ts";
import { num, round2, roundUpToNext10 } from "./numbers.ts";
import type { ProductCostLine } from "./types";

/**
 * The highest SP (before GST) the MRPs printed on bought-out items allow, in
 * whole rupees; null when no line has an MRP. MRP includes GST, so the items
 * may sell for MRP x qty / (1 + GST). In a product with other costs too, the
 * MRP items take their share of the SP by cost, and only that share is capped.
 */
export function mrpLimit(
  lines: Partial<ProductCostLine>[],
  overheads: StageOverheads | null | undefined,
  totalCost: number,
  gstRate: unknown,
): number | null {
  let ceiling = 0;
  let mrpCost = 0;
  for (const l of lines) {
    if (!(num(l.mrp) > 0)) continue;
    ceiling += num(l.mrp) * (l.qty == null ? 1 : num(l.qty));
    const stage = (l.stage_code ?? "").toLowerCase();
    mrpCost += calculateLineCost(l).line_total * (1 + num(overheads?.[stage]) / 100);
  }
  if (!ceiling) return null;
  const share = totalCost > 0 ? mrpCost / totalCost : 1;
  // A free MRP item takes none of the price, so it caps nothing.
  if (share <= 0) return null;
  return Math.floor(round2(ceiling / (1 + num(gstRate)) / share));
}

/**
 * An SP rounded up to the next 10, unless that rounding would take an SP
 * within the MRP limit over it. Math.round absorbs the paise of drift from a
 * markup % saved to 2 decimals.
 */
export function roundSp(sp: number, limit: number | null): number {
  const rounded = roundUpToNext10(sp);
  return limit != null && Math.round(sp) <= limit && rounded > limit ? limit : rounded;
}

/** The margin on one bought-out item sold at its MRP (less GST); null without an MRP or qty. */
export function mrpMargin(line: Partial<ProductCostLine>, gstRate: unknown): number | null {
  const qty = line.qty == null ? 1 : num(line.qty);
  if (!(num(line.mrp) > 0) || qty <= 0) return null;
  const maxSp = num(line.mrp) / (1 + num(gstRate));
  return (maxSp - calculateLineCost(line).line_total / qty) / maxSp;
}
