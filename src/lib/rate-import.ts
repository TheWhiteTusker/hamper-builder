/**
 * Reads the Rates & Hierarchy sheet (see rate-sheet.ts) back. Rows are matched
 * on their name path (any case), so renaming is done in the master screen.
 *
 * Pure: it works out what to create and update from the sheet and the current
 * tables, so a dry run can show it before anything is written.
 */
import type { CostRows } from "./cost-tree.ts";
import { body, cell, type Grid } from "./import-cells.ts";
import { num } from "./numbers.ts";
import type { CostVariety } from "./types.ts";

const NEW_UNIT = "sq ft"; // same defaults as the Add Variety dialog
const NEW_WASTAGE = 10;

type NewVariety = Omit<CostVariety, "id" | "subcategory_id" | "is_active"> & { subKey: string; label: string };

export type RatePlan = {
  /** Existing category / subcategory keys → ids; inserts add to it. */
  ids: Map<string, string>;
  newCategories: Map<string, { stage_id: string; name: string; sort_order: number }>;
  newSubcategories: Map<string, { catKey: string; name: string; sort_order: number }>;
  newVarieties: NewVariety[];
  updates: CostVariety[];
  reactivate: { categories: string[]; subcategories: string[] };
  /** Varieties whose rate or unit changed: their products need repricing. */
  reprice: string[];
  warnings: string[];
  counts: Record<string, number>;
};

const norm = (s: string) => s.trim().toLowerCase();
const list = (xs: string[]) => xs.slice(0, 8).join(", ") + (xs.length > 8 ? "…" : "");

/** `db` must hold every row, inactive ones too: names are unique per parent either way. */
export function planRateImport(grid: Grid, db: CostRows, onlyNew: boolean): RatePlan {
  const stages = new Map(db.stages.flatMap((s) => [[norm(s.name), s], [norm(s.code), s]] as const));
  const ids = new Map<string, string>();
  const keyOf = new Map<string, string>(); // category / subcategory id → key
  const inactive = new Set<string>();
  const nextSort = new Map<string, number>();
  const bumpSort = (parent: string, sort: number) => nextSort.set(parent, Math.max(nextSort.get(parent) ?? 0, sort + 1));
  const takeSort = (parent: string) => {
    const n = nextSort.get(parent) ?? 0;
    nextSort.set(parent, n + 1);
    return n;
  };

  for (const c of db.categories) {
    const k = `${c.stage_id}|${norm(c.name)}`;
    ids.set(k, c.id);
    keyOf.set(c.id, k);
    if (!c.is_active) inactive.add(c.id);
    bumpSort(c.stage_id, c.sort_order);
  }
  for (const s of db.subcategories) {
    const parent = keyOf.get(s.category_id);
    if (!parent) continue;
    const k = `${parent}|${norm(s.name)}`;
    ids.set(k, s.id);
    keyOf.set(s.id, k);
    if (!s.is_active) inactive.add(s.id);
    bumpSort(parent, s.sort_order);
  }
  const varieties = new Map<string, CostVariety>();
  for (const v of db.varieties) {
    const parent = keyOf.get(v.subcategory_id);
    if (!parent) continue;
    varieties.set(`${parent}|${norm(v.name)}`, v);
    bumpSort(parent, v.sort_order);
  }

  const plan: RatePlan = {
    ids,
    newCategories: new Map(),
    newSubcategories: new Map(),
    newVarieties: [],
    updates: [],
    reactivate: { categories: [], subcategories: [] },
    reprice: [],
    warnings: [],
    counts: {},
  };
  const incomplete: number[] = [];
  const unknownTypes = new Set<string>();
  const skipped: string[] = [];
  const repeated: string[] = [];
  const seen = new Set<string>();
  const reactivate = (key: string, into: string[]) => {
    const id = ids.get(key);
    if (id && inactive.delete(id)) into.push(id);
  };

  body(grid).forEach((row, i) => {
    const [type, cat, sub, name, rate, unit, wastage, notes] = Array.from({ length: 8 }, (_, c) => cell(row, c));
    if (!type || !cat || !sub || !name) return void incomplete.push(i + 2);
    const stage = stages.get(norm(type));
    if (!stage) return void unknownTypes.add(type);

    const catKey = `${stage.id}|${norm(cat)}`;
    if (ids.has(catKey)) reactivate(catKey, plan.reactivate.categories);
    else if (!plan.newCategories.has(catKey))
      plan.newCategories.set(catKey, { stage_id: stage.id, name: cat, sort_order: takeSort(stage.id) });

    const subKey = `${catKey}|${norm(sub)}`;
    if (ids.has(subKey)) reactivate(subKey, plan.reactivate.subcategories);
    else if (!plan.newSubcategories.has(subKey))
      plan.newSubcategories.set(subKey, { catKey, name: sub, sort_order: takeSort(catKey) });

    const key = `${subKey}|${norm(name)}`;
    const label = `${cat} › ${sub} › ${name}`;
    if (seen.has(key)) return void repeated.push(label);
    seen.add(key);

    const existing = varieties.get(key);
    if (!existing) {
      plan.newVarieties.push({
        subKey,
        label: `${stage.name} › ${label}`,
        name,
        default_rate: num(rate),
        unit: unit || NEW_UNIT,
        default_wastage_pct: wastage === "" ? NEW_WASTAGE : num(wastage),
        notes: notes || null,
        sort_order: takeSort(subKey),
      });
      return;
    }
    if (onlyNew) return void skipped.push(label);

    // A blank cell leaves that value as it is.
    const next: CostVariety = {
      ...existing,
      default_rate: rate === "" ? Number(existing.default_rate) : num(rate),
      unit: unit || existing.unit,
      default_wastage_pct: wastage === "" ? Number(existing.default_wastage_pct) : num(wastage),
      notes: notes || existing.notes,
      is_active: true,
    };
    const repricing = next.default_rate !== Number(existing.default_rate) || next.unit !== existing.unit;
    const changed =
      repricing ||
      next.default_wastage_pct !== Number(existing.default_wastage_pct) ||
      next.notes !== existing.notes ||
      !existing.is_active;
    if (changed) plan.updates.push(next);
    if (repricing) plan.reprice.push(existing.id);
  });

  if (incomplete.length)
    plan.warnings.push(`Rows missing a Type, Category, Subcategory or Variety were skipped: row ${list(incomplete.map(String))}`);
  if (unknownTypes.size)
    plan.warnings.push(
      `Unknown Type, rows skipped: ${list([...unknownTypes])}. Use one of: ${db.stages.filter((s) => s.is_active).map((s) => s.name).join(", ")}`,
    );
  if (repeated.length) plan.warnings.push(`Listed more than once, only the first is used: ${list(repeated)}`);
  if (skipped.length)
    plan.warnings.push(`${skipped.length} already exist and were skipped. Use “Edit & add rates” to change them: ${list(skipped)}`);
  if (plan.reprice.length)
    plan.warnings.push(`${plan.reprice.length} rate or unit change(s): products costed with them are repriced on save.`);

  plan.counts = {
    newCategories: plan.newCategories.size,
    newSubcategories: plan.newSubcategories.size,
    newVarieties: plan.newVarieties.length,
    updatedVarieties: plan.updates.length,
  };
  return plan;
}
