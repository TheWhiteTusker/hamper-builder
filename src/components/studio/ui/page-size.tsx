"use client";

import { useState } from "react";
import { cx, fieldCls } from "./base";

const PAGE_MIN = 100;
const PAGE_MAX = 8000;

/** Page width × height boxes: type a number, then Enter or click away to apply. */
export function PageSizeInputs({
  width,
  height,
  onChange,
}: {
  width: number;
  height: number;
  onChange: (width: number, height: number) => void;
}) {
  return (
    <div className="flex items-center gap-1" title={`Page size in pixels (${PAGE_MIN}–${PAGE_MAX})`}>
      <SizeBox label="W" name="Page width in pixels" value={width} onCommit={(w) => onChange(w, height)} />
      <span className="text-(--st-muted)">×</span>
      <SizeBox label="H" name="Page height in pixels" value={height} onCommit={(h) => onChange(width, h)} />
      <span className="text-[11px] text-(--st-muted)">px</span>
    </div>
  );
}

function SizeBox({ label, name, value, onCommit }: { label: string; name: string; value: number; onCommit: (v: number) => void }) {
  const [draft, setDraft] = useState<string | null>(null);
  const n = Number(draft);
  const invalid = draft !== null && !(/^\s*\d+\s*$/.test(draft) && n >= PAGE_MIN && n <= PAGE_MAX);

  const commit = () => {
    if (draft !== null && !invalid && n !== value) onCommit(n);
    setDraft(null); // an invalid entry snaps back to the current size
  };

  return (
    <label className="flex items-center gap-1">
      <span className="text-[11px] text-(--st-muted)">{label}</span>
      <input
        aria-label={name}
        inputMode="numeric"
        className={cx(fieldCls, "h-7 w-16 px-1.5 text-right tabular-nums", invalid && "border-red-600 focus:border-red-600")}
        value={draft ?? String(value)}
        onFocus={(e) => e.target.select()}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            setDraft(null);
            e.currentTarget.blur();
          }
        }}
      />
    </label>
  );
}
