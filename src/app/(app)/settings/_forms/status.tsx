"use client";

import { startTransition, type FormEvent } from "react";
import { markOtherPagesStale } from "@/components/stale-pages";
import type { ActionState } from "@/lib/forms";

export function Status({ state }: { state: ActionState }) {
  if (state.error)
    return (
      <span role="alert" className="text-sm text-red-700">
        {state.error}
      </span>
    );
  return null;
}

/**
 * onSubmit for a settings form. Saves no longer re-render the page, so this
 * skips React's reset after a form action (which would put the old values
 * back) and leaves what was typed on screen.
 */
export const keepValues = (action: (formData: FormData) => void) => (e: FormEvent<HTMLFormElement>) => {
  e.preventDefault();
  const formData = new FormData(e.currentTarget);
  startTransition(() => action(formData));
  markOtherPagesStale();
};
