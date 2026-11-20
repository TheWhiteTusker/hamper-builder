"use client";

import { useRef } from "react";
import { Plus, Search, X } from "lucide-react";

/** Search box for the selected list; Enter or the prompt below adds what isn't there. */
export function OptionSearch({
  value,
  onChange,
  listLabel,
  hasExactMatch,
  onAdd,
}: {
  value: string;
  onChange: (value: string) => void;
  listLabel: string;
  hasExactMatch: boolean;
  onAdd: (prefill: string) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const q = value.trim();

  return (
    <>
      <div className="relative max-w-md">
        <label className="label" htmlFor="picklist-search">
          Search options
        </label>
        <div className="relative mt-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-muted)" />
          <input
            ref={inputRef}
            id="picklist-search"
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && q && !hasExactMatch) {
                e.preventDefault();
                onAdd(q);
              }
            }}
            placeholder={`Search in ${listLabel.toLowerCase()}...`}
            className="input pl-9 pr-10"
          />
          {value && (
            <button
              type="button"
              onClick={() => {
                onChange("");
                inputRef.current?.focus();
              }}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 rounded-full p-1 text-(--color-muted) hover:bg-slate-100 hover:text-(--color-ink) transition-colors"
              title="Clear search"
              aria-label="Clear search text"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>

      {q && !hasExactMatch && (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-(--color-line) bg-paper px-3.5 py-2 text-xs">
          <span className="text-(--color-muted)">
            Not found in this list: <strong>&ldquo;{q}&rdquo;</strong>
          </span>
          <button
            type="button"
            onClick={() => onAdd(q)}
            className="inline-flex items-center gap-1 font-semibold text-(--color-brand) hover:underline"
          >
            <Plus className="h-3.5 w-3.5" />
            <span>Add &ldquo;{q}&rdquo; as new option</span>
          </button>
        </div>
      )}
    </>
  );
}
