"use client";

import { useEffect, useRef } from "react";
import { useRouter } from "next/navigation";
import { STEP_SIZE } from "@/lib/paging";

/**
 * Infinite scroll for the server-rendered lists: when this reaches the
 * bottom of the viewport it asks for the next STEP_SIZE rows by raising
 * ?limit=, so filters, refresh and back all keep what was loaded.
 *
 * ponytail: each step re-fetches every row shown so far, not just the next
 * page. Fine for a few hundred rows; switch to a server action returning only
 * the next page if a list grows into the thousands.
 */
export function LoadMore({ shown, total }: { shown: number; total: number }) {
  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);
  const more = shown < total;

  useEffect(() => {
    const el = ref.current;
    if (!more || !el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) return;
        // One request per step; the re-render with more rows re-arms it.
        observer.disconnect();
        const params = new URLSearchParams(window.location.search);
        params.set("limit", String(shown + STEP_SIZE));
        router.replace(`?${params}`, { scroll: false });
      },
      { rootMargin: "0px" },
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [more, shown, router]);

  if (!more) return null;

  return (
    <div ref={ref} className="py-4 text-center text-sm text-[var(--color-muted)]">
      Loading more…
    </div>
  );
}
