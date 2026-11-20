"use client";

import { useState, type ReactNode } from "react";
import { cx } from "./base";

/**
 * After Effects-style number: drag the label left/right to scrub the value
 * (Shift = ×10), or click the field and type.
 */
export function ScrubNumber({
  label,
  value,
  onChange,
  step = 1,
  min = -Infinity,
  max = Infinity,
  precision = 0,
  suffix,
  disabled,
}: {
  label: ReactNode;
  value: number;
  onChange: (v: number) => void;
  step?: number;
  min?: number;
  max?: number;
  precision?: number;
  suffix?: string;
  disabled?: boolean;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const tidy = (v: number) => Number(Math.min(max, Math.max(min, v)).toFixed(precision));

  const commit = () => {
    if (draft !== null && draft.trim() !== "" && Number.isFinite(Number(draft))) onChange(tidy(Number(draft)));
    setDraft(null);
  };

  return (
    <div className={cx("flex min-w-0 items-center gap-1.5", disabled && "opacity-40")}>
      <span
        className="shrink-0 cursor-ew-resize select-none text-[12px] text-(--st-muted) hover:text-(--st-accent)"
        onPointerDown={(e) => {
          if (disabled) return;
          e.preventDefault();
          const el = e.currentTarget;
          el.setPointerCapture(e.pointerId);
          const startX = e.clientX;
          const start = value;
          const move = (ev: PointerEvent) =>
            onChange(tidy(start + (ev.clientX - startX) * step * (ev.shiftKey ? 10 : 1)));
          const up = () => {
            el.removeEventListener("pointermove", move);
            el.removeEventListener("pointerup", up);
          };
          el.addEventListener("pointermove", move);
          el.addEventListener("pointerup", up);
        }}
      >
        {label}
      </span>
      <input
        disabled={disabled}
        inputMode="decimal"
        aria-label={typeof label === "string" ? label : undefined}
        className="h-7 w-full min-w-0 rounded border border-transparent bg-transparent px-1 text-[12px] tabular-nums text-(--st-value) outline-none hover:border-(--st-line) focus:border-(--st-accent) focus:bg-(--st-panel-2) focus:text-(--st-text)"
        value={draft ?? String(tidy(value))}
        onFocus={(e) => e.target.select()}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            setDraft(null);
            e.currentTarget.blur();
          }
          if (e.key === "ArrowUp" || e.key === "ArrowDown") {
            e.preventDefault();
            setDraft(null);
            onChange(tidy(value + (e.key === "ArrowUp" ? 1 : -1) * step * (e.shiftKey ? 10 : 1)));
          }
        }}
      />
      {suffix && <span className="shrink-0 text-[11px] text-(--st-muted)">{suffix}</span>}
    </div>
  );
}
