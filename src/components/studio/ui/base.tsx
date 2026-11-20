"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";

export const cx = (...c: (string | false | null | undefined)[]) => c.filter(Boolean).join(" ");

export const toolBtn =
  "inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-md px-2 text-[13px] text-(--st-text) " +
  "transition-colors hover:bg-(--st-hover) disabled:pointer-events-none disabled:opacity-35";
export const toolBtnActive = "bg-(--st-accent-soft) text-(--st-accent-strong) hover:bg-(--st-accent-soft)";
export const fieldCls =
  "h-8 rounded-md border border-(--st-line) bg-(--st-panel-2) px-2 text-[13px] text-(--st-text) " +
  "outline-none focus:border-(--st-accent)";
export const accentBtn =
  "inline-flex h-8 items-center justify-center gap-1.5 rounded-md bg-(--st-accent) px-3 text-[13px] font-medium " +
  "text-(--st-on-accent) hover:bg-(--st-accent-strong) disabled:opacity-50";
export const panelTitle = "text-[11px] font-semibold uppercase tracking-wider text-(--st-muted)";

export function ToolButton({
  title,
  active,
  className,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean }) {
  return (
    <button type="button" title={title} aria-label={title} className={cx(toolBtn, active && toolBtnActive, className)} {...rest}>
      {children}
    </button>
  );
}

export const Divider = () => <span className="mx-1 h-5 w-px shrink-0 bg-(--st-line)" />;

/** A toolbar button that opens a panel beneath it; closes on outside click or Escape. */
export function Popover({
  trigger,
  title,
  active,
  align = "left",
  width = 260,
  className,
  children,
}: {
  trigger: ReactNode;
  title: string;
  active?: boolean;
  align?: "left" | "right";
  width?: number;
  className?: string;
  children: ReactNode | ((close: () => void) => ReactNode);
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("pointerdown", down);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointerdown", down);
      window.removeEventListener("keydown", key);
    };
  }, [open]);

  return (
    <div ref={ref} className="relative">
      <ToolButton title={title} active={open || active} className={className} onClick={() => setOpen((o) => !o)}>
        {trigger}
      </ToolButton>
      {open && (
        <div
          data-popover
          className={cx(
            "absolute top-full z-50 mt-1.5 rounded-lg border border-(--st-line) bg-(--st-panel) p-3 shadow-xl shadow-[#2c332f]/15",
            align === "right" ? "right-0" : "left-0",
          )}
          style={{ width }}
        >
          {typeof children === "function" ? children(() => setOpen(false)) : children}
        </div>
      )}
    </div>
  );
}

/** A labelled range slider with its value shown alongside. */
export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  suffix = "",
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="block">
      <span className="flex justify-between text-[12px] text-(--st-muted)">
        {label}
        <span className="tabular-nums text-(--st-text)">
          {Number(value.toFixed(2))}
          {suffix}
        </span>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="mt-1 w-full"
      />
    </label>
  );
}
