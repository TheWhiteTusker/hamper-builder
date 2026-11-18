import { calculateLineCost, COMMON_UNITS } from "@/lib/costing.ts";
import { num } from "@/lib/pricing";
import type { CostStageWithHierarchy, ProductCostLine } from "@/lib/types";

export type LineState = ProductCostLine & { tempKey: string };

export const ORIGINS = ["In-house", "Outsource", "Hybrid"] as const;
export const BOUGHT_OUT = "bought_out";

/** Units a priced-by-size line can use; minutes and hours belong to the machine stage. */
export const LINE_UNITS = COMMON_UNITS.filter((u) => u !== "min" && u !== "hour");

/** The saved origin matched to one of ORIGINS, In-house when it is unknown. */
export const matchOrigin = (raw: string | null | undefined) =>
  ORIGINS.find((o) => o.toLowerCase() === (raw ?? "").toLowerCase()) ?? "In-house";

export function createEmptyLine(
  stageCode: string,
  categories: CostStageWithHierarchy["categories"],
): LineState {
  // Nothing preselected: the user picks category, subcategory and variety.
  const line: LineState = {
    id: crypto.randomUUID(),
    sheet_id: "",
    tempKey: crypto.randomUUID(),
    stage_code: stageCode,
    category_name: "",
    subcategory_name: "",
    variety_name: "",
    cost_variety_id: null,
    item_name: "",
    length: null,
    breadth: null,
    dimension_unit: "inch",
    unit:
      stageCode === "machine"
        ? "min"
        : stageCode === "bought_out"
          ? "piece"
          : categories.length > 0
            ? "sq ft"
            : "piece",
    rate: 0,
    qty: 1,
    duration_minutes: stageCode === "machine" ? 15 : null,
    wastage_pct: 0,
    sort_order: 0,
    calculated_area: 1,
    line_total: 0,
  };
  // The row must show the same amount the totals count for it.
  return { ...line, line_total: calculateLineCost(line).line_total };
}

/** The fields of a line that are sent to the server on save. */
export const toSavedLine = (l: LineState): ProductCostLine => ({
  stage_code: l.stage_code,
  category_name: l.category_name,
  subcategory_name: l.subcategory_name,
  variety_name: l.variety_name,
  item_name: l.item_name,
  cost_variety_id: l.cost_variety_id,
  length: l.length,
  breadth: l.breadth,
  dimension_unit: l.dimension_unit,
  unit: l.unit,
  rate: l.rate,
  duration_minutes: l.duration_minutes,
  qty: l.qty,
  wastage_pct: l.wastage_pct,
  calculated_area: l.calculated_area,
  line_total: l.line_total,
  mrp: l.mrp,
});

/** The lines a save sends: starter rows left empty are skipped. */
export const savedLines = (lines: LineState[]) =>
  lines.filter((l) => l.category_name || l.item_name || num(l.rate) > 0).map(toSavedLine);

/**
 * A saved line brought up to its master variety: current names, rate, unit
 * and wastage. Found by the variety's id, so a rename in the master still matches.
 */
export function withMaster(line: LineState, stages: CostStageWithHierarchy[]): LineState {
  if (!line.cost_variety_id) return line;
  for (const stage of stages) {
    for (const cat of stage.categories) {
      for (const sub of cat.subcategories) {
        const v = sub.varieties.find((x) => x.id === line.cost_variety_id);
        if (!v) continue;
        const next: LineState = {
          ...line,
          category_name: cat.name,
          subcategory_name: sub.name,
          variety_name: v.name,
          item_name: `${sub.name} ${v.name}`,
          rate: Number(v.default_rate),
          unit: v.unit,
          wastage_pct: line.stage_code === BOUGHT_OUT ? 0 : Number(v.default_wastage_pct),
        };
        const calc = calculateLineCost(next);
        return { ...next, calculated_area: calc.calculated_area, line_total: calc.line_total };
      }
    }
  }
  return line;
}
