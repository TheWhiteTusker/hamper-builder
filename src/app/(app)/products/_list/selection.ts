"use client";

import { useSyncExternalStore } from "react";

// Ticked products, kept in this browser until unticked: they survive other
// screens, reloads and filters. Per browser, not per account.
const KEY = "lattice:selected-products";
const EVENT = "lattice:selection";
const EMPTY: string[] = [];

let raw: string | null = null;
let ids: string[] = EMPTY;

function read(): string[] {
  let next: string | null = null;
  try {
    next = localStorage.getItem(KEY);
  } catch {}
  if (next !== raw) {
    raw = next;
    try {
      ids = next ? JSON.parse(next) : EMPTY;
    } catch {
      ids = EMPTY;
    }
  }
  return ids;
}

function write(next: string[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
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

export function useSelectedProducts() {
  return useSyncExternalStore(subscribe, read, () => EMPTY);
}

export function setSelected(id: string, on: boolean) {
  const cur = read().filter((x) => x !== id);
  write(on ? [...cur, id] : cur);
}

export function clearSelected() {
  write([]);
}
