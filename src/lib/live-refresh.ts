// Live updates: one Realtime channel (components/live-refresh.tsx) hands every
// database change to the pages listening here, and decides whether it needs
// a background refresh. Pure, so it's tested without a browser.

type Row = Record<string, unknown>;

/** One row change as Supabase Realtime (postgres_changes) sends it. */
export type LiveChange = {
  table: string;
  eventType: "INSERT" | "UPDATE" | "DELETE";
  new: Row;
  old: Row;
};

// Replaced on every save of their parent, whose own row changes in the same save.
const CHILD_TABLES = new Set(["hamper_items", "quote_items", "presentation_slides"]);

/**
 * True when someone else made the change, so this page needs a refresh.
 * A delete carries only the row's id, so it can't say who: child-table
 * deletes are skipped (the parent's update says it), the rest refresh.
 */
export function isOthersChange(c: LiveChange, userId: string) {
  if (c.eventType === "DELETE") return !CHILD_TABLES.has(c.table);
  return c.new.updated_by !== userId;
}

const listeners = new Set<(c: LiveChange) => void>();

/** Hear every change. Returns the unsubscribe. */
export function onLiveChange(fn: (c: LiveChange) => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

export function emitLiveChange(c: LiveChange) {
  for (const fn of listeners) fn(c);
}

/** Replace rows by id, append new ones, drop removed ids. Untouched rows keep their identity. */
export function patchById<T extends { id: string }>(list: T[], upsert: T[] = [], remove: string[] = []) {
  const byId = new Map(upsert.map((r) => [r.id, r]));
  const next = list.filter((r) => !remove.includes(r.id)).map((r) => byId.get(r.id) ?? r);
  const known = new Set(list.map((r) => r.id));
  return [...next, ...upsert.filter((r) => !known.has(r.id))];
}

/**
 * A list after one change. Rows not in the list are left to the background
 * refresh, which knows the page's filters, order and joined columns. Returns
 * the same array when nothing on it changed, so React skips the render.
 */
export function applyRowChange<T extends { id: string }>(list: T[], c: LiveChange, merge: (cur: T, row: Row) => T): T[] {
  const id = (c.eventType === "DELETE" ? c.old.id : c.new.id) as string | undefined;
  const i = list.findIndex((r) => r.id === id);
  if (i < 0) return list;
  if (c.eventType === "DELETE" || c.new.deleted_at) return list.filter((_, j) => j !== i);
  const next = [...list];
  next[i] = merge(list[i], c.new);
  return next;
}
