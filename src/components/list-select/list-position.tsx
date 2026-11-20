"use client";

import { useEffect, useSyncExternalStore } from "react";
import { useSearchParams } from "next/navigation";
import Link from "@/components/link";
import type { ListKind } from "./selection";

// A list's last URL (filters + ?limit= rows loaded) and scroll spot, so
// "← Products" / "← Hampers" lands where you left off. Per tab.
const key = (kind: ListKind) => `lattice:list-${kind}`;
const restoreKey = (kind: ListKind) => `lattice:list-${kind}-restore`;

/** The rows scroll inside this box (globals.css), not the window. */
const box = () => document.querySelector<HTMLElement>(".list-scroll");

type Spot = { url: string; y: number };
const noop = () => () => {};

function load(kind: ListKind): Spot | null {
  try {
    return JSON.parse(sessionStorage.getItem(key(kind)) ?? "null");
  } catch {
    return null;
  }
}

/** Next visit to the list scrolls back to the remembered spot. */
export function markRestore(kind: ListKind) {
  try {
    sessionStorage.setItem(restoreKey(kind), "1");
  } catch {}
}

/** On the list page (/products, /hampers): keeps the spot up to date, and returns to it after "Back". */
export function RememberListPosition({ kind }: { kind: ListKind }) {
  // Filters, Clear and load-more change the URL without remounting this, so
  // the spot is re-saved on every change, not just on mount or scroll.
  const search = useSearchParams().toString();

  useEffect(() => {
    const url = () => location.pathname + location.search;
    try {
      const spot = load(kind);
      if (sessionStorage.getItem(restoreKey(kind)) && spot?.url === url()) box()?.scrollTo(0, spot.y);
      sessionStorage.removeItem(restoreKey(kind));
    } catch {}

    const save = () => {
      // Navigating away scrolls the next page to the top after the URL has
      // changed; ignore that so it doesn't overwrite the spot.
      if (location.pathname !== `/${kind}`) return;
      try {
        sessionStorage.setItem(key(kind), JSON.stringify({ url: url(), y: box()?.scrollTop ?? 0 }));
      } catch {}
    };
    save();
    // Scroll doesn't bubble; capture catches the box's, even after it's re-rendered.
    document.addEventListener("scroll", save, { capture: true, passive: true });
    return () => document.removeEventListener("scroll", save, { capture: true });
  }, [kind, search]);

  return null;
}

/** The list as it was left: its filters and rows loaded. */
export function useListHref(kind: ListKind) {
  return useSyncExternalStore(
    noop,
    () => load(kind)?.url ?? `/${kind}`,
    () => `/${kind}`,
  );
}

/** Hidden field telling a save where to land: the list as it was left. */
export function ListReturnField({ kind }: { kind: ListKind }) {
  return <input type="hidden" name="return_to" value={useListHref(kind)} />;
}

/** "← Products" / "← Hampers" on a detail page, back to the list as it was. */
export function BackToList({ kind }: { kind: ListKind }) {
  return (
    <Link
      href={useListHref(kind)}
      scroll={false}
      onClick={() => markRestore(kind)}
      className="mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)"
    >
      <span aria-hidden>←</span> {kind === "products" ? "Products" : "Hampers"}
    </Link>
  );
}
