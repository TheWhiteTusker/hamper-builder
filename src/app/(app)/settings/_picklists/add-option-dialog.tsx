"use client";

import { useEffect, useRef, useState } from "react";
import { AlertCircle, Loader2, Plus, X } from "lucide-react";

/** "Add new option" pop-up. Refuses blanks and duplicates before calling onAdd. */
export function AddOptionDialog({
  listLabel,
  initial,
  existing,
  isPending,
  onAdd,
  onClose,
}: {
  listLabel: string;
  initial: string;
  existing: string[];
  isPending: boolean;
  onAdd: (value: string) => void;
  onClose: () => void;
}) {
  const [value, setValue] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 50);
  }, []);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = value.trim();
    if (!trimmed) return setError("Please enter an option name.");
    if (existing.some((item) => item.toLowerCase() === trimmed.toLowerCase())) {
      return setError(`"${trimmed}" already exists in ${listLabel}.`);
    }
    onAdd(trimmed);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs transition-opacity animate-in fade-in duration-150"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="add-modal-title"
        className="card w-full max-w-md p-5 shadow-xl transition-all scale-100"
      >
        <div className="flex items-start justify-between border-b border-(--color-line) pb-3">
          <div>
            <h3 id="add-modal-title" className="text-base font-semibold text-(--color-ink)">
              Add new option
            </h3>
            <p className="mt-0.5 text-xs text-(--color-muted)">
              Adding to <strong>{listLabel}</strong>
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full p-1 text-(--color-muted) hover:bg-slate-100 hover:text-(--color-ink) transition-colors"
            aria-label="Close dialog"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          <div>
            <label className="label" htmlFor="new-picklist-item">
              Option value
            </label>
            <input
              ref={inputRef}
              id="new-picklist-item"
              type="text"
              required
              value={value}
              onChange={(e) => {
                setValue(e.target.value);
                if (error) setError(null);
              }}
              placeholder="e.g. Festival Special"
              className="input mt-1"
            />
            {error && (
              <p className="mt-1.5 flex items-center gap-1 text-xs text-red-600">
                <AlertCircle className="h-3.5 w-3.5 shrink-0" />
                <span>{error}</span>
              </p>
            )}
          </div>

          <div className="flex items-center justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="btn-secondary" disabled={isPending}>
              Cancel
            </button>
            <button type="submit" className="btn-primary gap-1.5" disabled={isPending || !value.trim()}>
              {isPending ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>Saving…</span>
                </>
              ) : (
                <>
                  <Plus className="h-3.5 w-3.5" />
                  <span>Add option</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
