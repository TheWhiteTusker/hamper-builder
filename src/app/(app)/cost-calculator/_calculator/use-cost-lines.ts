import { useMemo, useState } from "react";
import { calculateLineCost } from "@/lib/costing.ts";
import type { CostStageWithHierarchy, ProductCostLine } from "@/lib/types";
import { BOUGHT_OUT, createEmptyLine, withMaster, type LineState } from "./lines";

export type CostLines = ReturnType<typeof useCostLines>;

/** The cost lines being edited, with every edit recomputing the line's total. */
export function useCostLines(stages: CostStageWithHierarchy[], initialLines?: ProductCostLine[]) {
  const [lines, setLines] = useState<LineState[]>(() =>
    initialLines?.length
      ? // Saved lines show the master's current names and rates (see withMaster).
        initialLines.map((l) => withMaster({ ...l, tempKey: crypto.randomUUID() }, stages))
      : // One empty row per stage, ready to be filled from the dropdowns.
        stages.map((s) => createEmptyLine(s.code, s.categories)),
  );
  const stageMap = useMemo(() => new Map(stages.map((s) => [s.code, s])), [stages]);

  const recalc = (l: LineState) => {
    const calc = calculateLineCost(l);
    return { ...l, calculated_area: calc.calculated_area, line_total: calc.line_total };
  };

  function update(key: string, patch: Partial<LineState>) {
    setLines((prev) => prev.map((l) => (l.tempKey === key ? recalc({ ...l, ...patch }) : l)));
  }

  // A rate's wastage is the rate's, not the line's: it changes on every line of
  // that rate, and on the Rates page when the costing is saved.
  function setWastage(line: LineState, wastage_pct: number) {
    const id = line.cost_variety_id;
    setLines((prev) => prev.map((l) => (l.tempKey === line.tempKey || (id && l.cost_variety_id === id) ? recalc({ ...l, wastage_pct }) : l)));
  }

  // Category changed: clear subcategory and variety so each is picked in turn.
  function setCategory(key: string, catName: string) {
    update(key, {
      category_name: catName,
      subcategory_name: "",
      variety_name: "",
      item_name: "",
      cost_variety_id: null,
      rate: 0,
      wastage_pct: 0,
    });
  }

  // Subcategory changed: clear the variety; picking one fills rate and unit.
  function setSubcategory(key: string, subName: string) {
    update(key, {
      subcategory_name: subName,
      variety_name: "",
      item_name: subName,
      cost_variety_id: null,
      rate: 0,
      wastage_pct: 0,
    });
  }

  // A variety (e.g. 8mm -> 12mm) brings its default rate, unit and wastage.
  function setVariety(line: LineState, varName: string) {
    const cat = stageMap.get(line.stage_code)?.categories.find((c) => c.name === line.category_name);
    const sub = cat?.subcategories.find((s) => s.name === line.subcategory_name);
    const v = sub?.varieties.find((x) => x.name === varName);
    const item_name = sub ? `${sub.name} ${v?.name ?? varName}` : varName;

    update(
      line.tempKey,
      v
        ? {
            variety_name: varName,
            item_name,
            cost_variety_id: v.id,
            unit: v.unit,
            rate: v.default_rate,
            wastage_pct: line.stage_code === BOUGHT_OUT ? 0 : v.default_wastage_pct,
          }
        : { variety_name: varName, item_name, cost_variety_id: null },
    );
  }

  // Duplicate a line directly below it
  function duplicate(key: string) {
    setLines((prev) => {
      const idx = prev.findIndex((l) => l.tempKey === key);
      if (idx < 0) return prev;
      const next = [...prev];
      next.splice(idx + 1, 0, { ...prev[idx], tempKey: crypto.randomUUID() });
      return next;
    });
  }

  const remove = (key: string) => setLines((prev) => prev.filter((l) => l.tempKey !== key));

  function addToStage(stageCode: string) {
    const stage = stageMap.get(stageCode);
    if (stage) setLines((prev) => [...prev, createEmptyLine(stageCode, stage.categories)]);
  }

  // Bought-out items: a finished item purchased in, priced rate x qty; only
  // the sheet-level markup applies.
  const addBoughtOut = () => addToStage(BOUGHT_OUT);

  return { lines, update, setWastage, setCategory, setSubcategory, setVariety, duplicate, remove, addToStage, addBoughtOut };
}
