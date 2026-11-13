"use client";

import { useEffect, useRef } from "react";
import Link from "@/components/link";

export type MenuItem = { href: string; title: string; hint: string; /** A file download, not a page. */ download?: boolean };

/**
 * A button that opens a short list of actions. Native <details>, so it opens
 * from the keyboard with no extra code; closes on an outside click or Escape.
 */
export function Menu({ label, items, align = "right" }: { label: string; items: MenuItem[]; align?: "left" | "right" }) {
  const ref = useRef<HTMLDetailsElement>(null);

  useEffect(() => {
    const close = (e: Event) => {
      const el = ref.current;
      if (!el?.open) return;
      if (e instanceof KeyboardEvent ? e.key === "Escape" : !el.contains(e.target as Node)) el.open = false;
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", close);
    };
  }, []);

  const itemClass = "block rounded-md px-3 py-2 hover:bg-paper focus:bg-paper focus:outline-none";
  const body = (i: MenuItem) => (
    <>
      <span className="block text-sm font-medium text-(--color-ink)">{i.title}</span>
      <span className="block text-xs text-(--color-muted)">{i.hint}</span>
    </>
  );

  return (
    <details ref={ref} className="relative">
      <summary className="btn-secondary cursor-pointer list-none [&::-webkit-details-marker]:hidden">
        {label}
        <span aria-hidden className="text-xs">▾</span>
      </summary>
      <div
        className={`absolute z-20 mt-1 w-64 rounded-lg border border-line bg-white p-1 shadow-lg ${align === "right" ? "right-0" : "left-0"}`}
      >
        {items.map((i) =>
          i.download ? (
            <a key={i.href} href={i.href} className={itemClass} onClick={() => ref.current?.removeAttribute("open")}>
              {body(i)}
            </a>
          ) : (
            <Link key={i.href} href={i.href} className={itemClass}>
              {body(i)}
            </Link>
          ),
        )}
      </div>
    </details>
  );
}
