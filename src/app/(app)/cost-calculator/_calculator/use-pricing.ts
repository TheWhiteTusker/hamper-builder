import { useMemo, useState } from "react";
import { calculateCostSheetTotals } from "@/lib/costing";
import { mrpLimit, roundSp } from "@/lib/mrp";
import { num, round2, roundUpToNext10 } from "@/lib/pricing";
import type { Product, ProductCostSheet } from "@/lib/types";
import type { LineState } from "./lines";

export type Pricing = ReturnType<typeof usePricing>;

/**
 * Live totals, with markup % and selling price each updating the other.
 * Each stage's overhead % is added to that stage's subtotal. Bought-out MRPs
 * (GST included) set the highest SP; rounding never crosses it.
 */
export function usePricing(
  activeLines: LineState[],
  initialProduct?: Product | null,
  initialSheet?: ProductCostSheet | null,
  gstRate = 0,
) {
  const [markupPct, setMarkupPct] = useState(
    String(initialProduct?.markup_pct ?? initialSheet?.markup_pct ?? "50"),
  );

  // Stage code -> overhead % as typed.
  const [overheads, setOverheads] = useState<Record<string, string>>(() =>
    Object.fromEntries(Object.entries(initialSheet?.stage_overheads ?? {}).map(([k, v]) => [k, String(v)])),
  );
  const overheadFor = (stageCode: string) => overheads[stageCode.toLowerCase()] ?? "";
  const setOverhead = (stageCode: string, pct: string) =>
    setOverheads((prev) => ({ ...prev, [stageCode.toLowerCase()]: pct }));

  const totals = useMemo(
    () => calculateCostSheetTotals(activeLines, num(markupPct), overheads),
    [activeLines, markupPct, overheads],
  );
  // The highest SP the bought-out items' MRPs allow; null when none has one.
  const mrpMax = useMemo(
    () => mrpLimit(activeLines, overheads, totals.total_cost, gstRate),
    [activeLines, overheads, totals.total_cost, gstRate],
  );
  /** The SP a markup gives, or null at 100%+ where there is no finite price. */
  const spAt = (pct: unknown) => {
    const divisor = 1 - num(pct) / 100;
    return divisor > 0 ? roundSp(totals.total_cost / divisor, mrpMax) : null;
  };

  const [manualSp, setManualSp] = useState(() => {
    // Saved lines are refreshed from the current rate master before they reach
    // this hook. Start with a price calculated from those same current rates,
    // just as changing the markup does, instead of briefly showing a stale
    // catalogue price. Keep the saved price for products without costing lines.
    if (totals.total_cost > 0) return String(spAt(markupPct) ?? totals.calculated_sp);
    const savedSp = roundUpToNext10(initialProduct?.default_sp ?? initialSheet?.calculated_sp ?? 0);
    return savedSp > 0 ? String(savedSp) : "";
  });

  const effectiveSp = manualSp ? roundSp(num(manualSp), mrpMax) : (spAt(markupPct) ?? totals.calculated_sp);
  const effectiveMargin = effectiveSp > 0 ? (effectiveSp - totals.total_cost) / effectiveSp : 0;
  const overMrp = mrpMax != null && effectiveSp > mrpMax;

  function changeMarkup(val: string) {
    setMarkupPct(val);
    const sp = spAt(val);
    // 100%+ has no finite SP; leave the price alone rather than show Infinity
    if (sp != null) setManualSp(String(sp));
  }

  function changeSp(val: string) {
    setManualSp(val);
    const sp = num(val);
    if (totals.total_cost > 0 && sp > 0) {
      setMarkupPct(String(round2(((sp - totals.total_cost) / sp) * 100)));
    }
  }

  function roundManualSp() {
    if (manualSp) {
      const rounded = roundSp(num(manualSp), mrpMax);
      if (rounded > 0 && String(rounded) !== manualSp) {
        setManualSp(String(rounded));
        if (totals.total_cost > 0) {
          setMarkupPct(String(round2(((rounded - totals.total_cost) / rounded) * 100)));
        }
      }
    }
  }

  /** The overheads to save: numbers, blanks and zeros left out. */
  const stageOverheads = Object.fromEntries(
    Object.entries(overheads).flatMap(([k, v]) => (num(v) ? [[k, num(v)]] : [])),
  );

  return {
    markupPct, manualSp, totals, effectiveSp, effectiveMargin, changeMarkup, changeSp, roundManualSp,
    overheadFor, setOverhead, stageOverheads,
    mrpMax, overMrp, setSpToMrp: () => mrpMax != null && changeSp(String(mrpMax)),
  };
}
