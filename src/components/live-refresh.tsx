"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { createClient } from "@/lib/supabase/client";

// Someone saves anywhere → open pages re-fetch. Which tables count is set by
// the supabase_realtime publication (migration 0027). Changes within a second
// fold into one refresh, and it waits while the tab is hidden or a text field
// has focus, so it never lands mid-typing.
//
// ponytail: your own saves echo back as one extra refresh. Stamp an
// updated_by column and skip your own rows if Worker requests ever matter.

const TYPING = "input:not([type=checkbox],[type=radio],[type=button],[type=submit]), textarea, [contenteditable=true]";

export function LiveRefresh() {
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
      timer = setTimeout(flush, 1000);
    };
    // Focus has moved by the next tick; only refresh if it left text fields.
    const leftField = () => setTimeout(flush);

    const channel = supabase
      .channel("live-refresh")
      .on("postgres_changes", { event: "*", schema: "public" }, changed)
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
  }, [router]);

  return null;
}
