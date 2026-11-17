"use client";

import { useEffect, useSyncExternalStore } from "react";
import Link from "@/components/link";

// The products list's last URL (filters + ?limit= rows loaded) and scroll
// spot, so "Back to products" lands where you left off. Per tab.
const KEY = "lattice:products-list";
const RESTORE = "lattice:products-list-restore";

type Spot = { url: string; y: number };
const noop = () => () => {};

function load(): Spot | null {
  try {
    return JSON.parse(sessionStorage.getItem(KEY) ?? "null");
  } catch {
    return null;
  }
}

/** On the list page: keeps the spot up to date, and returns to it after "Back". */
export function RememberListPosition() {
  useEffect(() => {
    const url = () => location.pathname + location.search;
    try {
      const spot = load();
      if (sessionStorage.getItem(RESTORE) && spot?.url === url()) window.scrollTo(0, spot.y);
      sessionStorage.removeItem(RESTORE);
    } catch {}

    const save = () => {
      // Navigating away scrolls the next page to the top after the URL has
      // changed; ignore that so it doesn't overwrite the spot.
      if (location.pathname !== "/products") return;
      try {
        sessionStorage.setItem(KEY, JSON.stringify({ url: url(), y: window.scrollY }));
      } catch {}
    };
    save();
    window.addEventListener("scroll", save, { passive: true });
    return () => window.removeEventListener("scroll", save);
  }, []);

  return null;
}

/** "← Products" on a product page, back to the list as it was. */
export function BackToProducts() {
  const href = useSyncExternalStore(
    noop,
    () => load()?.url ?? "/products",
    () => "/products",
  );

  return (
    <Link
      href={href}
      scroll={false}
      onClick={() => {
        try {
          sessionStorage.setItem(RESTORE, "1");
        } catch {}
      }}
      className="mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)"
    >
      <span aria-hidden>←</span> Products
    </Link>
  );
}
