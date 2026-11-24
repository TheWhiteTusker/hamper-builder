"use client";

import { useSyncExternalStore } from "react";
import { record } from "@/lib/undo";

// Ticked rows of a list (products, hampers), kept in this browser until
// unticked: they survive other screens, reloads and filters. Per browser,
// not per account.
export type ListKind = "products" | "hampers";
/** A list's ticked rows, or its rows bookmarked for review (same store, own key). */
export type Store = ListKind | `review-${ListKind}`;

const key = (kind: Store) => `lattice:selected-${kind}`;
const EVENT = "lattice:selection";
const EMPTY: string[] = [];

// One parsed copy per list, so the snapshot only changes when storage does.
const cache = new Map<Store, { raw: string | null; ids: string[] }>();

function read(kind: Store): string[] {
  let raw: string | null = null;
  try {
    raw = localStorage.getItem(key(kind));
  } catch {}
  const hit = cache.get(kind);
  if (hit && hit.raw === raw) return hit.ids;
  let ids = EMPTY;
  try {
    ids = raw ? JSON.parse(raw) : EMPTY;
  } catch {}
  cache.set(kind, { raw, ids });
  return ids;
}

function write(kind: Store, next: string[]) {
  try {
    localStorage.setItem(key(kind), JSON.stringify(next));
  } catch {}
  // Desktop app: save to disk now. Chromium holds writes in memory for 5s+,
  // so an installer or crash killing the app lost the latest ticks.
  (window as { electron?: { flushStorage?: () => void } }).electron?.flushStorage?.();
  window.dispatchEvent(new Event(EVENT));
}

function subscribe(cb: () => void) {
  window.addEventListener(EVENT, cb);
  window.addEventListener("storage", cb); // other tabs
  return () => {
    window.removeEventListener(EVENT, cb);
    window.removeEventListener("storage", cb);
  };
}

export function useSelected(kind: Store) {
  return useSyncExternalStore(subscribe, () => read(kind), () => EMPTY);
}

// A row is ticked or marked for review, never both: doing one undoes the other.
const RIVAL: Partial<Record<Store, Store>> = {
  products: "review-products",
  "review-products": "products",
  hampers: "review-hampers",
  "review-hampers": "hampers",
};

/** Every tick, mark and clear goes through here, so each is one undo step. */
function change(next: Partial<Record<Store, string[]>>) {
  const stores = Object.keys(next) as Store[];
  const prev = Object.fromEntries(stores.map((s) => [s, read(s)])) as Record<Store, string[]>;
  const apply = (to: Partial<Record<Store, string[]>>) => stores.forEach((s) => write(s, to[s]!));
  apply(next);
  record({ undo: () => apply(prev), redo: () => apply(next) });
}

export function setSelected(kind: Store, id: string, on: boolean) {
  const cur = read(kind).filter((x) => x !== id);
  const rival = RIVAL[kind];
  change({
    [kind]: on ? [...cur, id] : cur,
    ...(on && rival && read(rival).includes(id) ? { [rival]: read(rival).filter((x) => x !== id) } : {}),
  });
}

export function clearSelected(kind: Store) {
  change({ [kind]: [] });
}
