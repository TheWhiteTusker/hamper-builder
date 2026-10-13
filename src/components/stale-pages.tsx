"use client";

import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";

// Screens that patch their own rows after a save skip revalidatePath, because
// it re-renders the whole current page on the Worker. Other pages may still be
// in the browser's 2-minute page cache, so the next navigation refreshes once,
// which also empties that cache.
let stale = false;

/** Call after a save that only patched the page in place. */
export function markOtherPagesStale() {
  stale = true;
}

export function RefreshStalePages() {
  const pathname = usePathname();
  const router = useRouter();

  useEffect(() => {
    if (!stale) return;
    stale = false;
    router.refresh();
  }, [pathname, router]);

  return null;
}
