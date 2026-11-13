"use client";

import { useState, useTransition } from "react";
import { syncAllRates } from "./sync-actions";

/** One click to bring every saved product costing up to the master's current rates. */
export function SyncRatesButton() {
  const [pending, startTransition] = useTransition();
  const [result, setResult] = useState<{ error?: string; message?: string } | null>(null);

  function sync() {
    if (
      !confirm(
        "Update every product costed with an old rate? Their cost and selling price are recalculated (each keeps its markup). Quotes and invoices don't change.",
      )
    )
      return;
    setResult(null);
    startTransition(async () => setResult(await syncAllRates()));
  }

  return (
    <div className="card flex flex-wrap items-center justify-between gap-3 p-4">
      <div>
        <h3 className="text-base font-semibold text-[var(--color-ink)]">Products on old rates</h3>
        <p className="mt-0.5 text-xs text-[var(--color-muted)]">
          Changing a rate updates products automatically. Use this for products costed before a change.
        </p>
        {result?.message && <p className="mt-1.5 text-xs font-medium text-emerald-700">{result.message}</p>}
        {result?.error && <p className="mt-1.5 text-xs font-medium text-red-700">{result.error}</p>}
      </div>
      <button type="button" onClick={sync} disabled={pending} className="btn-secondary text-xs">
        {pending ? "Updating…" : "Update products"}
      </button>
    </div>
  );
}
