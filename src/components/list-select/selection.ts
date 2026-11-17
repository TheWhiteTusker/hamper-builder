"use client";

import { useSyncExternalStore } from "react";

// Ticked rows of a list (products, hampers), kept in this browser until
// unticked: they survive other screens, reloads and filters. Per browser,
// not per account.
export type ListKind = "products" | "hampers";

const key = (kind: ListKind) => `lattice:selected-${kind}`;
const EVENT = "lattice:selection";
const EMPTY: string[] = [];

// One parsed copy per list, so the snapshot only changes when storage does.
const cache = new Map<ListKind, { raw: string | null; ids: string[] }>();

function read(kind: ListKind): string[] {
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

function write(kind: ListKind, next: string[]) {
  try {
    localStorage.setItem(key(kind), JSON.stringify(next));
  } catch {}
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

export function useSelected(kind: ListKind) {
  return useSyncExternalStore(subscribe, () => read(kind), () => EMPTY);
}

export function setSelected(kind: ListKind, id: string, on: boolean) {
  const cur = read(kind).filter((x) => x !== id);
  write(kind, on ? [...cur, id] : cur);
}

export function clearSelected(kind: ListKind) {
  write(kind, []);
}
