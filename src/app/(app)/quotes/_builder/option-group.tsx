"use client";

import { useState, type ReactNode } from "react";
import { formatMoney, type QuotePricing } from "@/lib/pricing";

/**
 * One option of an option-based quote: its name, its own pickers and lines,
 * and its own total. The name commits on blur, so typing a new name doesn't
 * regroup the lines on every keystroke.
 */
export function OptionGroup({
  label,
  totals,
  gstPct,
  canEdit,
  pickers,
  onRename,
  onRemove,
  children,
}: {
  label: string;
  totals: QuotePricing;
  gstPct: string;
  canEdit: boolean;
  pickers: ReactNode;
  onRename: (to: string) => void;
  onRemove: () => void;
  children: ReactNode;
}) {
  const [draft, setDraft] = useState(label);
  const [prevLabel, setPrevLabel] = useState(label);
  if (label !== prevLabel) {
    setPrevLabel(label);
    setDraft(label);
  }
  const commit = () => {
    const next = draft.trim();
    if (next && next !== label) onRename(next);
    else setDraft(label);
  };

  return (
    <div className="border-t border-[var(--color-line)] first:border-t-0">
      <div className="flex flex-wrap items-end justify-between gap-3 bg-[var(--color-brand-soft)] px-4 py-2.5">
        <div className="flex items-end gap-2">
          <div>
            <label className="label" htmlFor={`opt-${label}`}>
              Option name
            </label>
            <input
              id={`opt-${label}`}
              className="input mt-1 max-w-[180px] font-semibold"
              value={draft}
              disabled={!canEdit}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={commit}
              onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), commit())}
            />
          </div>
          {canEdit && (
            <button type="button" onClick={onRemove} className="btn-secondary text-xs text-red-700">
              Remove option
            </button>
          )}
        </div>
        {canEdit && <div className="flex flex-wrap items-end gap-2">{pickers}</div>}
      </div>

      {children}

      <dl className="ml-auto flex max-w-[520px] flex-wrap justify-end gap-x-6 gap-y-1 px-4 py-2.5 text-sm">
        <Total label="Subtotal" value={formatMoney(totals.subtotal)} />
        <Total label={`GST ${gstPct || 0}%`} value={formatMoney(totals.gstAmount)} />
        <Total label={`${label} total`} value={formatMoney(totals.grandTotal)} strong />
      </dl>
    </div>
  );
}

function Total({ label, value, strong }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className="flex items-baseline gap-2">
      <dt className="text-[var(--color-muted)]">{label}</dt>
      <dd className={`tabular-nums ${strong ? "text-base font-semibold" : "font-medium"}`}>{value}</dd>
    </div>
  );
}
