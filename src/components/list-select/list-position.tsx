"use client";

import { useEffect, useSyncExternalStore } from "react";
import Link from "@/components/link";
import type { ListKind } from "./selection";

// A list's last URL (filters + ?limit= rows loaded) and scroll spot, so
// "← Products" / "← Hampers" lands where you left off. Per tab.
const key = (kind: ListKind) => `lattice:list-${kind}`;
const restoreKey = (kind: ListKind) => `lattice:list-${kind}-restore`;

type Spot = { url: string; y: number };
const noop = () => () => {};

function load(kind: ListKind): Spot | null {
  try {
    return JSON.parse(sessionStorage.getItem(key(kind)) ?? "null");
  } catch {
    return null;
  }
}

/** On the list page (/products, /hampers): keeps the spot up to date, and returns to it after "Back". */
export function RememberListPosition({ kind }: { kind: ListKind }) {
  useEffect(() => {
    const url = () => location.pathname + location.search;
    try {
      const spot = load(kind);
      if (sessionStorage.getItem(restoreKey(kind)) && spot?.url === url()) window.scrollTo(0, spot.y);
      sessionStorage.removeItem(restoreKey(kind));
    } catch {}

    const save = () => {
      // Navigating away scrolls the next page to the top after the URL has
      // changed; ignore that so it doesn't overwrite the spot.
      if (location.pathname !== `/${kind}`) return;
      try {
        sessionStorage.setItem(key(kind), JSON.stringify({ url: url(), y: window.scrollY }));
      } catch {}
    };
    save();
    window.addEventListener("scroll", save, { passive: true });
    return () => window.removeEventListener("scroll", save);
  }, [kind]);

  return null;
}

/** "← Products" / "← Hampers" on a detail page, back to the list as it was. */
export function BackToList({ kind }: { kind: ListKind }) {
  const href = useSyncExternalStore(
    noop,
    () => load(kind)?.url ?? `/${kind}`,
    () => `/${kind}`,
  );

  return (
    <Link
      href={href}
      scroll={false}
      onClick={() => {
        try {
          sessionStorage.setItem(restoreKey(kind), "1");
        } catch {}
      }}
      className="mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)"
    >
      <span aria-hidden>←</span> {kind === "products" ? "Products" : "Hampers"}
    </Link>
  );
}
