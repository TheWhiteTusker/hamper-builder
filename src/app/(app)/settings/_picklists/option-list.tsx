"use client";

import { Loader2, Plus, X } from "lucide-react";

/** The options in the selected list as removable chips, or an empty state. */
export function OptionList({
  items,
  total,
  query,
  listLabel,
  isPending,
  onAdd,
  onRemove,
}: {
  items: string[];
  total: number;
  /** The trimmed search text, "" when not searching. */
  query: string;
  listLabel: string;
  isPending: boolean;
  onAdd: (prefill?: string) => void;
  onRemove: (item: string) => void;
}) {
  return (
    <div className="rounded-lg border border-(--color-line) bg-white p-3.5">
      <div className="mb-2.5 flex items-center justify-between text-xs text-(--color-muted)">
        <span>
          {items.length} {items.length === 1 ? "item" : "items"}
          {query && ` matching "${query}" (out of ${total})`}
        </span>
        {isPending && (
          <span className="flex items-center gap-1 text-(--color-brand)">
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
            Saving changes…
          </span>
        )}
      </div>

      {items.length === 0 ? (
        <div className="py-8 text-center text-sm">
          {query ? (
            <div>
              <p className="text-(--color-muted)">No matching options for &ldquo;{query}&rdquo;</p>
              <button type="button" onClick={() => onAdd(query)} className="btn-primary mt-3 gap-1.5">
                <Plus className="h-4 w-4" />
                <span>Add &ldquo;{query}&rdquo; now</span>
              </button>
            </div>
          ) : (
            <div>
              <p className="text-(--color-muted)">No options in {listLabel} yet.</p>
              <button type="button" onClick={() => onAdd()} className="btn-secondary mt-3 gap-1.5">
                <Plus className="h-4 w-4" />
                <span>Add the first option</span>
              </button>
            </div>
          )}
        </div>
      ) : (
        <div className="flex flex-wrap gap-2">
          {items.map((item) => (
            <div
              key={item}
              className="group inline-flex items-center gap-1.5 rounded-full border border-(--color-line) bg-paper py-1 pl-3 pr-1.5 text-xs text-(--color-ink) transition-colors hover:border-slate-300 hover:bg-slate-100"
            >
              <span className="font-medium">{item}</span>
              <button
                type="button"
                onClick={() => onRemove(item)}
                disabled={isPending}
                title={`Remove "${item}"`}
                aria-label={`Remove ${item}`}
                className="rounded-full p-0.5 text-(--color-muted) hover:bg-red-100 hover:text-red-700 transition-colors disabled:opacity-40"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
