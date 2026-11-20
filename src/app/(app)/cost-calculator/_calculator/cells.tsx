"use client";

import { useState } from "react";
import { COMMON_UNITS } from "@/lib/costing.ts";
import { mrpMargin } from "@/lib/mrp";
import { formatMoney, formatPct, num } from "@/lib/pricing";
import type { ProductCostLine } from "@/lib/types";
import { LINE_UNITS } from "./lines";

/** A number input in a line table. `nullable` turns a cleared box into null rather than 0. */
export function NumCell({
  value,
  onChange,
  nullable,
  placeholder,
  min,
  mono,
  title,
}: {
  value: number | null | undefined;
  onChange: (value: number | null) => void;
  nullable?: boolean;
  placeholder?: string;
  min?: string;
  mono?: boolean;
  title?: string;
}) {
  return (
    <td className="p-2">
      <input
        title={title}
        type="number"
        min={min}
        step="any"
        value={value ?? ""}
        onChange={(e) => onChange(nullable && !e.target.value ? null : Number(e.target.value))}
        placeholder={placeholder}
        className={`input input-num text-xs py-1 px-2${mono ? " font-mono" : ""}`}
      />
    </td>
  );
}

/** The pricing unit of a line, keeping an old unit that is no longer in the list. */
export function UnitCell({ value, onChange }: { value: string; onChange: (unit: string) => void }) {
  return (
    <td className="p-2">
      <select value={value} onChange={(e) => onChange(e.target.value)} className="select text-xs py-1 px-2">
        {LINE_UNITS.map((u) => (
          <option key={u} value={u}>
            {u}
          </option>
        ))}
        {value && !COMMON_UNITS.includes(value as (typeof COMMON_UNITS)[number]) && (
          <option value={value}>{value}</option>
        )}
      </select>
    </td>
  );
}

/**
 * A bought-out item's printed MRP (per piece, GST included): a "+ MRP" button
 * until one is typed, then the MRP with the margin it leaves at that price.
 */
export function MrpCell({
  line,
  gstRate,
  onChange,
}: {
  line: ProductCostLine;
  gstRate: number;
  onChange: (mrp: number | null) => void;
}) {
  // Keeps the box open while it is cleared mid-typing.
  const [open, setOpen] = useState(false);
  if (!open && line.mrp == null) {
    return (
      <td className="p-2 text-center">
        <button
          type="button"
          onClick={() => setOpen(true)}
          title="This item has an MRP printed on it"
          className="rounded border border-(--color-border) px-2 py-1 text-[11px] font-semibold text-(--color-brand) transition-colors hover:bg-emerald-50"
        >
          + MRP
        </button>
      </td>
    );
  }
  const margin = mrpMargin(line, gstRate);
  return (
    <td className="p-2">
      <input
        type="number"
        min="0"
        step="any"
        autoFocus={open}
        value={line.mrp ?? ""}
        onChange={(e) => onChange(Number(e.target.value) > 0 ? Number(e.target.value) : null)}
        onBlur={() => setOpen(false)}
        placeholder="MRP"
        aria-label="MRP per piece, GST included"
        className="input input-num font-mono text-xs py-1 px-2"
      />
      {margin != null && (
        <div className={`mt-0.5 whitespace-nowrap text-[10px] ${margin > 0 ? "text-emerald-700" : "font-semibold text-red-600"}`}>
          Max SP {formatMoney(num(line.mrp) / (1 + gstRate))} · {formatPct(margin, 0)} margin
        </div>
      )}
    </td>
  );
}

export function RemoveButton({ onClick, title }: { onClick: () => void; title: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className="rounded px-1.5 py-1 text-[11px] font-semibold text-red-600 transition-colors hover:bg-red-50"
    >
      &times;
    </button>
  );
}

/** Coloured header strip shared by the stage cards and Bought Out Items. */
export function SectionHeader({
  badge,
  title,
  hint,
  totalLabel,
  total,
  lead,
  extra,
}: {
  badge: React.ReactNode;
  title: string;
  hint: string;
  totalLabel: string;
  total: string;
  lead?: React.ReactNode;
  /** Shown just before the total, e.g. the overhead % box. */
  extra?: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border)] bg-[var(--color-sheet)] px-4 py-3">
      <div className="flex items-center gap-3">
        {lead}
        {badge}
        <h3 className="text-base font-semibold text-[var(--color-ink)]">{title}</h3>
        <span className="text-xs text-[var(--color-muted)]">{hint}</span>
      </div>
      <div className="flex items-center gap-3">
        {extra}
        <span className="text-xs text-[var(--color-muted)]">{totalLabel}</span>
        <span className="rounded-lg bg-white px-2.5 py-1 font-mono text-sm font-bold text-[var(--color-brand-dark)] shadow-sm">
          {total}
        </span>
      </div>
    </div>
  );
}

/** Overhead % for one stage, and the amount it adds to the stage's lines. */
export function OverheadInput({
  stageName,
  value,
  added,
  onChange,
}: {
  stageName: string;
  value: string;
  /** The overhead amount, already formatted; hidden while it is zero. */
  added: string | null;
  onChange: (pct: string) => void;
}) {
  return (
    <label className="flex items-center gap-1.5 text-xs text-[var(--color-muted)]" title={`Overhead added to the ${stageName} subtotal`}>
      Overhead
      <span className="flex items-center rounded-lg border border-[var(--color-border)] bg-white pr-1.5 shadow-xs">
        <input
          type="number"
          min="0"
          step="any"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="0"
          aria-label={`${stageName} overhead percent`}
          className="w-14 rounded-lg bg-transparent px-1.5 py-1 text-right font-mono text-xs text-[var(--color-ink)] outline-none"
        />
        %
      </span>
      {added && <span className="font-mono text-[var(--color-brand-dark)]">+{added}</span>}
    </label>
  );
}
