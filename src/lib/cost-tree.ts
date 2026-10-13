import type {
  CostCategory,
  CostStage,
  CostStageWithHierarchy,
  CostSubcategory,
  CostVariety,
} from "./types";

/** The costing hierarchy as the four flat tables it is stored in. */
export type CostRows = {
  stages: CostStage[];
  categories: CostCategory[];
  subcategories: CostSubcategory[];
  varieties: CostVariety[];
};

type Editable = {
  categories: CostCategory;
  subcategories: CostSubcategory;
  varieties: CostVariety;
};

/** Rows a save wrote (replaced by id, or added) and ids it deleted, for one table. */
export type CostChange = {
  [T in keyof Editable]: { table: T; upsert?: Editable[T][]; remove?: string[] };
}[keyof Editable];

/** What a Rates & Hierarchy Master action returns. */
export type CostSaveResult = { ok?: boolean; error?: string; message?: string; changes?: CostChange[] };

function groupBy<T>(rows: T[], key: (row: T) => string) {
  const map = new Map<string, T[]>();
  for (const row of rows) map.set(key(row), [...(map.get(key(row)) ?? []), row]);
  return map;
}

// Stable, so rows added locally sit after existing ones with the same sort_order.
const sorted = <T extends { sort_order: number }>(rows: T[]) =>
  [...rows].sort((a, b) => a.sort_order - b.sort_order);

/** stage -> category -> subcategory -> variety. */
export function buildCostTree(rows: CostRows): CostStageWithHierarchy[] {
  const varietiesBySub = groupBy(sorted(rows.varieties), (v) => v.subcategory_id);
  const subsByCat = groupBy(
    sorted(rows.subcategories).map((sub) => ({ ...sub, varieties: varietiesBySub.get(sub.id) ?? [] })),
    (sub) => sub.category_id,
  );
  const catsByStage = groupBy(
    sorted(rows.categories).map((cat) => ({ ...cat, subcategories: subsByCat.get(cat.id) ?? [] })),
    (cat) => cat.stage_id,
  );
  return sorted(rows.stages).map((stage) => ({ ...stage, categories: catsByStage.get(stage.id) ?? [] }));
}

function patch<T extends { id: string }>(list: T[], upsert: T[] = [], remove: string[] = []) {
  const byId = new Map(upsert.map((r) => [r.id, r]));
  const next = list.filter((r) => !remove.includes(r.id)).map((r) => byId.get(r.id) ?? r);
  const known = new Set(list.map((r) => r.id));
  return [...next, ...upsert.filter((r) => !known.has(r.id))];
}

/** The rows after a save, touching only what the save changed. */
export function applyCostChanges(rows: CostRows, changes: CostChange[]): CostRows {
  const next = { ...rows };
  for (const c of changes) {
    // Each branch keeps the row type tied to its table.
    if (c.table === "categories") next.categories = patch(next.categories, c.upsert, c.remove);
    else if (c.table === "subcategories") next.subcategories = patch(next.subcategories, c.upsert, c.remove);
    else next.varieties = patch(next.varieties, c.upsert, c.remove);
  }
  return next;
}
