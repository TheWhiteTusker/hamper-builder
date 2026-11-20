"use client";

import { useEffect, useRef } from "react";
import { useSearchParams } from "next/navigation";
import Form from "next/form";

/**
 * The filter bar applies itself: a pick applies at once, typing applies after
 * a short pause. Submits as a GET to this page, replacing the history entry so
 * Back skips the keystrokes.
 */
export function FilterForm({ className, children }: { className?: string; children: React.ReactNode }) {
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const form = useRef<HTMLFormElement>(null);
  const search = useSearchParams().toString();

  // The URL changed from outside the bar (the nav's "Products" link, Back):
  // show its filters, not the old text. Never while typing in the bar.
  useEffect(() => {
    if (form.current && !form.current.contains(document.activeElement)) form.current.reset();
  }, [search]);

  return (
    <Form
      ref={form}
      action=""
      replace
      scroll={false}
      className={className}
      onInput={(e) => {
        const form = e.currentTarget;
        const typing = (e.target as HTMLInputElement).type === "text";
        clearTimeout(timer.current);
        timer.current = setTimeout(() => form.requestSubmit(), typing ? 350 : 0);
      }}
    >
      {children}
    </Form>
  );
}
