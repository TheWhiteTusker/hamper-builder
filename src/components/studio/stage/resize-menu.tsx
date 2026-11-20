"use client";

import { useState } from "react";
import { CANVAS_PRESETS } from "@/lib/hamper-canvas";
import { accentBtn, cx, fieldCls, panelTitle } from "../studio-ui";

/** Width × height boxes in the Resize menu. */
function CustomSize({ width, height, onApply }: { width: number; height: number; onApply: (w: number, h: number) => void }) {
  const [w, setW] = useState(String(width));
  const [h, setH] = useState(String(height));
  const valid = (v: string) => /^\d+$/.test(v.trim()) && Number(v) >= 100 && Number(v) <= 8000;
  const ok = valid(w) && valid(h);
  const box = cx(fieldCls, "w-18 text-right tabular-nums");

  return (
    <form
      className="mb-1 border-b border-(--st-line) px-2 pb-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (ok) onApply(Number(w), Number(h));
      }}
    >
      <div className={cx(panelTitle, "mb-1.5")}>Custom size</div>
      <div className="flex items-center gap-1.5">
        <input aria-label="Page width in pixels" inputMode="numeric" className={box} value={w} onChange={(e) => setW(e.target.value)} />
        <span className="text-(--st-muted)">×</span>
        <input aria-label="Page height in pixels" inputMode="numeric" className={box} value={h} onChange={(e) => setH(e.target.value)} />
        <span className="text-[11px] text-(--st-muted)">px</span>
        <button type="submit" className={cx(accentBtn, "ml-auto px-2.5")} disabled={!ok}>
          Apply
        </button>
      </div>
      <p className="mt-1.5 text-[11px] leading-snug text-(--st-muted)">
        100–8000 px. You can also drag the handles on the page&apos;s right and bottom edges.
      </p>
    </form>
  );
}

/** Custom size plus the preset page sizes. */
export function ResizeMenu({
  width,
  height,
  onResize,
}: {
  width: number;
  height: number;
  onResize: (w: number, h: number) => void;
}) {
  return (
    <div className="space-y-1">
      <CustomSize width={width} height={height} onApply={onResize} />
      <div className={cx(panelTitle, "px-2 pb-1 pt-1")}>Presets</div>
      {CANVAS_PRESETS.map((p) => (
        <button
          key={p.label}
          type="button"
          onClick={() => onResize(p.width, p.height)}
          className={cx(
            "flex w-full items-center justify-between rounded px-2 py-1.5 hover:bg-(--st-hover)",
            p.width === width && p.height === height && "text-(--st-accent)",
          )}
        >
          {p.label}
          <span className="text-[11px] tabular-nums text-(--st-muted)">
            {p.width}×{p.height}
          </span>
        </button>
      ))}
    </div>
  );
}
