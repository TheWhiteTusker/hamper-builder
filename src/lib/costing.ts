import { num, round2, roundUpToNext10 } from "./numbers.ts";
import { dimensionArea, type DimensionUnit } from "./dimensions.ts";
import type { ProductCostLine } from "./types";

export * from "./dimensions.ts";

/** Time units a rate can be quoted in, as minutes per unit. */
const MINUTES_PER: Record<string, number> = { min: 1, mins: 1, minute: 1, minutes: 1, hour: 60, hr: 60, hours: 60 };

/** A rate quoted per minute or per hour: priced on duration, not size. */
export const isTimeUnit = (unit: string | null | undefined) => (unit ?? "").trim().toLowerCase() in MINUTES_PER;

const LINEAR_UNITS = ["feet", "ft", "running ft", "rft", "inch", "in", "mm", "cm", "meter", "m"];
const AREA_UNITS = ["sq ft", "sqft", "sft", "sq inch", "sqinch", "sq in", "sqin", "sq mm", "sqmm", "sq cm", "sqcm"];

/** A rate priced on length or area, so the line's dimensions count. Blank means sq ft. */
export const isSizedUnit = (unit: string | null | undefined) => {
  const u = (unit || "sq ft").trim().toLowerCase();
  return LINEAR_UNITS.includes(u) || AREA_UNITS.includes(u);
};

/**
 * Computes the line total cost for a cost item line.
 */
export function calculateLineCost(line: Partial<ProductCostLine>): {
  calculated_area: number;
  line_total: number;
} {
  const stage = (line.stage_code ?? "").toLowerCase();
  const rate = num(line.rate);
  // Blank qty means 1; an explicit 0 means the line costs nothing.
  const qty = line.qty == null ? 1 : num(line.qty);
  // Bought-out items take only the sheet-level markup, never a per-line one.
  const wastage = stage === "bought_out" ? 0 : num(line.wastage_pct);

  // Machine time: minutes x the rate per minute (or per hour / 60). A machine
  // rate quoted by size (e.g. engraving per sq inch) is priced on dimensions below.
  const machineUnit = line.unit || "min";
  if (stage === "machine" && isTimeUnit(machineUnit)) {
    const duration = num(line.duration_minutes);
    const baseCost = (duration / MINUTES_PER[machineUnit.trim().toLowerCase()]) * rate * qty;
    const total = baseCost * (1 + wastage / 100);
    return {
      calculated_area: duration,
      line_total: round2(total),
    };
  }

  const dimUnit = (line.dimension_unit as DimensionUnit) || "inch";
  const unit = line.unit || "sq ft";
  const l = num(line.length);
  const b = num(line.breadth);

  const isLinearUnit = LINEAR_UNITS.includes(unit.trim().toLowerCase());
  const isAreaUnit = AREA_UNITS.includes(unit.trim().toLowerCase());

  let area = 1;
  let baseCost = 0;

  // The exact area is costed; it is rounded only for display.
  if ((isLinearUnit && l > 0) || (isAreaUnit && l > 0 && b > 0)) {
    const exact = dimensionArea(l, b, dimUnit, unit);
    area = round2(exact);
    baseCost = exact * rate * qty;
  } else {
    // Piece-based or no dimension multiplication
    baseCost = rate * qty;
  }

  const total = baseCost * (1 + wastage / 100);

  return {
    calculated_area: area,
    line_total: round2(total),
  };
}

export type CostSheetTotals = {
  material_total: number;
  hardware_total: number;
  finishing_total: number;
  machine_total: number;
  other_total: number;
  total_cost: number;
  markup_pct: number;
  calculated_sp: number;
  target_margin: number;
};

/**
 * A product's selling price after its cost changes: scaled with the cost so
 * its margin holds, including a price that was set by hand. Without a
 * previous cost and price there is nothing to scale, so `fallbackSp` is used.
 */
export function scaledPrice(oldCost: unknown, oldSp: unknown, newCost: number, fallbackSp: number) {
  const cost = num(oldCost);
  const sp = num(oldSp);
  return cost > 0 && sp > 0 ? roundUpToNext10((sp * newCost) / cost) : roundUpToNext10(fallbackSp);
}

/** Overhead % per stage code, e.g. { material: 10 }; added on top of that stage's lines. */
export type StageOverheads = Record<string, unknown>;

/** A stage's subtotal with its overhead % added. */
export const withOverhead = (amount: number, overheadPct: unknown) => round2(amount * (1 + num(overheadPct) / 100));

/**
 * Aggregates all lines by stage, adds each stage's overhead %, and
 * calculates selling price and margin.
 */
export function calculateCostSheetTotals(
  lines: Partial<ProductCostLine>[],
  markupPct: unknown = 0,
  overheads: StageOverheads = {},
): CostSheetTotals {
  let material_total = 0;
  let hardware_total = 0;
  let finishing_total = 0;
  let machine_total = 0;
  let other_total = 0;

  for (const line of lines) {
    const stage = (line.stage_code ?? "").toLowerCase();
    // Scaling each line by its stage's overhead is the same as scaling the stage subtotal.
    const line_total = calculateLineCost(line).line_total * (1 + num(overheads[stage]) / 100);

    if (stage === "material") {
      material_total += line_total;
    } else if (stage === "hardware") {
      hardware_total += line_total;
    } else if (stage === "finishing") {
      finishing_total += line_total;
    } else if (stage === "machine") {
      machine_total += line_total;
    } else {
      // miscellaneous, bought_out, or any stage added later
      other_total += line_total;
    }
  }

  material_total = round2(material_total);
  hardware_total = round2(hardware_total);
  finishing_total = round2(finishing_total);
  machine_total = round2(machine_total);
  other_total = round2(other_total);

  const total_cost = round2(
    material_total + hardware_total + finishing_total + machine_total + other_total,
  );

  const mPct = num(markupPct);
  // SP = CP / (1 - markup% / 100). A markup of 100% or more has no finite
  // selling price, so it falls back to cost price rather than Infinity/NaN.
  // Selling prices round off to the next 10 (e.g. 271.50 -> 280).
  const divisor = 1 - mPct / 100;
  const calculated_sp = divisor > 0 ? roundUpToNext10(total_cost / divisor) : roundUpToNext10(total_cost);
  const target_margin = calculated_sp > 0 ? (calculated_sp - total_cost) / calculated_sp : 0;

  return {
    material_total,
    hardware_total,
    finishing_total,
    machine_total,
    other_total,
    total_cost,
    markup_pct: mPct,
    calculated_sp,
    target_margin: round2(target_margin),
  };
}
