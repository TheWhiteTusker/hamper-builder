"use client";

import { codesForColors, type ProductColor } from "@/lib/product-code";

const chip = (on: boolean) =>
  `rounded-full font-medium transition-all ${
    on ? "bg-[var(--color-brand)] text-white shadow-sm" : "bg-[var(--color-sheet)] text-[var(--color-muted)] hover:bg-slate-200"
  }`;

/** Source / vendor with quick-pick chips. */
export function SourceField({ value, onChange, sources }: { value: string; onChange: (v: string) => void; sources: string[] }) {
  return (
    <div className="sm:col-span-2">
      <label className="label" htmlFor="source">
        Source / vendor
      </label>
      <input id="source" name="source" value={value} onChange={(e) => onChange(e.target.value)} className="input mt-1" />
      <div className="mt-1.5 flex flex-wrap gap-1.5">
        {sources.map((s) => (
          <button key={s} type="button" onClick={() => onChange(s)} className={`px-2.5 py-0.5 text-xs ${chip(value === s)}`}>
            {s}
          </button>
        ))}
      </div>
    </div>
  );
}

/** The finishes a product comes in. A new product in several saves as one product per colour. */
export function ColorPicker({
  allColors,
  selected,
  code,
  isNew,
  onToggle,
}: {
  allColors: ProductColor[];
  selected: string[];
  code: string;
  isNew: boolean;
  onToggle: (color: string) => void;
}) {
  return (
    <div className="sm:col-span-2">
      <div className="flex items-center justify-between">
        <label className="label">Available Colors / Finishes</label>
        <span className="text-xs text-[var(--color-muted)]">Manage the list in Cost Calculator → Rates</span>
      </div>
      <div className="mt-1.5 flex flex-wrap gap-2">
        {allColors.map((col) => {
          const checked = selected.includes(col.name);
          return (
            <button
              key={col.name}
              type="button"
              onClick={() => onToggle(col.name)}
              className={`flex items-center gap-2 px-3 py-1 text-xs ${chip(checked)}`}
            >
              <span
                className="inline-block h-2.5 w-2.5 rounded-full border border-black/20 bg-slate-300"
                style={{ backgroundColor: col.hex }}
              />
              <span>
                {col.name} ({col.code})
              </span>
              {checked && <span>✓</span>}
            </button>
          );
        })}
      </div>
      {isNew && selected.length > 1 && (
        <p className="mt-1.5 text-xs text-[var(--color-muted)]">
          Creates {selected.length} products:{" "}
          <span className="font-mono">
            {codesForColors(code, selected)
              .map((c) => c.code)
              .join(", ")}
          </span>
        </p>
      )}
    </div>
  );
}
