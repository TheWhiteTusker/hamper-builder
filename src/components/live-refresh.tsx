"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import { emitLiveChange, isOthersChange, type LiveChange } from "@/lib/live-refresh";

// Someone saves anywhere → every change goes to the pages patching rows in
// place (use-live.ts), then open pages re-fetch in the background. Which
// tables count is set by the supabase_realtime publication (migration 0027).
// Changes within 300 ms fold into one refresh, and it waits while the tab is
// hidden or a text field has focus, so it never lands mid-typing. Your own
// saves don't refresh: the saving page already shows them.
//
// ponytail: your own hard deletes outside child tables (bin purge, client or
// master delete) still echo one refresh, and pages that don't patch rows
// don't live-update your own second window (it catches up on navigation).

const TYPING = "input:not([type=checkbox],[type=radio],[type=button],[type=submit]), textarea, [contenteditable=true]";

export function LiveRefresh({ userId }: { userId: string }) {
  const router = useRouter();

  useEffect(() => {
    const supabase = createClient();
    let timer: ReturnType<typeof setTimeout> | undefined;
    let pending = false;
    let joined = false;

    const flush = () => {
      if (!pending || document.hidden || document.activeElement?.matches(TYPING)) return;
      pending = false;
      router.refresh();
    };
    const changed = () => {
      pending = true;
      clearTimeout(timer);
      timer = setTimeout(flush, 300);
    };
    const received = (c: LiveChange) => {
      emitLiveChange(c);
      if (isOthersChange(c, userId)) changed();
    };
    // Focus has moved by the next tick; only refresh if it left text fields.
    const leftField = () => setTimeout(flush);

    const channel = supabase
      .channel("live-refresh")
      .on("postgres_changes", { event: "*", schema: "public" }, (c) => received(c as LiveChange))
      .subscribe((status) => {
        // Rejoining after sleep or a dropped connection: changes were missed.
        if (status === "SUBSCRIBED" && joined) changed();
        if (status === "SUBSCRIBED") joined = true;
      });
    document.addEventListener("visibilitychange", flush);
    document.addEventListener("focusout", leftField);

    return () => {
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", flush);
      document.removeEventListener("focusout", leftField);
      supabase.removeChannel(channel);
    };
  }, [router, userId]);

  return null;
}
