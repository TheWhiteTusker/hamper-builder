"use client";

import { useRef } from "react";
import Form from "next/form";

/**
 * The filter bar applies itself: a pick applies at once, typing applies after
 * a short pause. Submits as a GET to this page, replacing the history entry so
 * Back skips the keystrokes.
 */
export function FilterForm({ className, children }: { className?: string; children: React.ReactNode }) {
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  return (
    <Form
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
