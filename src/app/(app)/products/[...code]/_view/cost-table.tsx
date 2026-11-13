import { isSizedUnit, isTimeUnit, withOverhead } from "@/lib/costing.ts";
import { formatMoney, num } from "@/lib/pricing";
import type { ProductCostLine } from "@/lib/types";

const DIM_LABEL: Record<string, string> = { inch: "″", mm: " mm", cm: " cm" };

/** What was measured: the size for sized lines, minutes for machine time. */
function size(l: ProductCostLine) {
  if (l.stage_code === "machine" && isTimeUnit(l.unit || "min")) return `${num(l.duration_minutes)} min`;
  if (!isSizedUnit(l.unit)) return "—";
  const u = DIM_LABEL[l.dimension_unit ?? "inch"] ?? "";
  if (num(l.length) > 0 && num(l.breadth) > 0) return `${num(l.length)}${u} × ${num(l.breadth)}${u}`;
  if (num(l.length) > 0) return `${num(l.length)}${u}`;
  return "—";
}

/**
 * One cost type (or Bought Out Items) as saved: a header with its total and
 * any overhead %, and its lines, all read-only.
 */
export function CostTable({
  badge,
  title,
  lines,
  overhead,
}: {
  badge: string;
  title: string;
  lines: ProductCostLine[];
  overhead?: number;
}) {
  const linesTotal = lines.reduce((sum, l) => sum + num(l.line_total), 0);
  const total = withOverhead(linesTotal, overhead);

  return (
    <div className="card overflow-hidden shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border)] bg-[var(--color-sheet)] px-4 py-3">
        <div className="flex items-center gap-3">
          <span className="flex h-7 w-7 items-center justify-center rounded-full bg-[var(--color-brand)] text-xs font-bold text-white">
            {badge}
          </span>
          <h3 className="text-base font-semibold text-[var(--color-ink)]">{title}</h3>
          <span className="text-xs text-[var(--color-muted)]">
            {lines.length} line{lines.length === 1 ? "" : "s"}
          </span>
        </div>
        <div className="flex items-center gap-3 text-xs text-[var(--color-muted)]">
          {num(overhead) > 0 && (
            <span>
              Overhead {num(overhead)}%{" "}
              <span className="font-mono text-[var(--color-brand-dark)]">+{formatMoney(total - linesTotal)}</span>
            </span>
          )}
          <span>Total:</span>
          <span className="rounded-lg bg-white px-2.5 py-1 font-mono text-sm font-bold text-[var(--color-brand-dark)] shadow-sm">
            {formatMoney(total)}
          </span>
        </div>
      </div>

      <div className="overflow-x-auto p-2">
        <table className="w-full text-left text-xs">
          <thead>
            <tr className="border-b border-[var(--color-border)] font-semibold text-[var(--color-muted)]">
              <th className="p-2 min-w-[240px]">Item</th>
              <th className="p-2 min-w-[110px]">Size / Time</th>
              <th className="p-2 min-w-[110px]">Rate / Unit</th>
              <th className="p-2 text-right">Qty</th>
              <th className="p-2 text-right">Wastage</th>
              <th className="p-2 text-right min-w-[95px]">Total Cost</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--color-border)]">
            {lines.map((l, i) => (
              <tr key={l.id ?? i}>
                <td className="p-2">
                  <div className="font-semibold text-[var(--color-ink)]">{l.variety_name || l.item_name || "—"}</div>
                  {l.variety_name && (l.category_name || l.subcategory_name) && (
                    <div className="text-[11px] text-[var(--color-muted)]">
                      {[l.category_name, l.subcategory_name].filter(Boolean).join(" › ")}
                    </div>
                  )}
                </td>
                <td className="p-2 text-[var(--color-muted)]">{size(l)}</td>
                <td className="p-2 whitespace-nowrap font-mono">
                  {formatMoney(l.rate)}/{l.unit}
                </td>
                <td className="p-2 text-right tabular-nums">{num(l.qty)}</td>
                <td className="p-2 text-right tabular-nums text-[var(--color-muted)]">
                  {num(l.wastage_pct) ? `${num(l.wastage_pct)}%` : "—"}
                </td>
                <td className="p-2 text-right font-mono font-semibold text-[var(--color-ink)]">
                  {formatMoney(l.line_total)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
