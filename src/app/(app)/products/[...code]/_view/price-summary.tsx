import type { CostSheetTotals } from "@/lib/costing.ts";
import { formatMoney, formatPct, num } from "@/lib/pricing";
import { CostBreakdown } from "../../../cost-calculator/_calculator/cost-breakdown";

/** The calculator's breakdown and price boxes, read-only. */
export function PriceSummary({
  totals,
  costPrice,
  markupPct,
  sellingPrice,
  mrpMax,
  notes,
}: {
  /** Stage totals from the saved lines, for the breakdown bar; null with no costing. */
  totals: CostSheetTotals | null;
  costPrice: number;
  markupPct: number | null | undefined;
  sellingPrice: number;
  /** The highest SP its bought-out MRPs allow; null without one. */
  mrpMax?: number | null;
  notes: string | null | undefined;
}) {
  const sp = num(sellingPrice);
  const cp = num(costPrice);
  const boxes: [string, string, string][] = [
    ["Total Cost Price (CP)", formatMoney(cp), "text-[var(--color-ink)]"],
    ["Markup %", markupPct != null ? `${num(markupPct)}%` : "—", "text-[var(--color-ink)]"],
    ["Selling Price (SP)", formatMoney(sp), "text-[var(--color-brand-dark)]"],
    ["Margin", sp > 0 ? formatPct((sp - cp) / sp, 1) : "—", "text-emerald-700"],
  ];

  return (
    <div className="card p-5 bg-gradient-to-br from-white to-slate-50 border-2 border-[var(--color-brand)]/20 shadow-md">
      <h3 className="text-base font-bold text-[var(--color-ink)] mb-4">Cost Breakdown & Selling Price</h3>
      {totals && <CostBreakdown totals={totals} />}

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {boxes.map(([label, value, tone]) => (
          <div key={label} className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
            <span className="text-xs font-semibold text-[var(--color-muted)] uppercase tracking-wider">{label}</span>
            <div className={`mt-1 font-mono text-2xl font-black ${tone}`}>{value}</div>
          </div>
        ))}
      </div>
      {mrpMax != null && (
        <p className={`mt-2 text-xs ${sp > mrpMax ? "font-semibold text-red-700" : "text-(--color-muted)"}`}>
          {sp > mrpMax ? "Above MRP limit" : "MRP limit"}: max SP {formatMoney(mrpMax)} (MRP includes GST)
        </p>
      )}

      {notes && (
        <div className="mt-4">
          <div className="label">Costing notes / specifications</div>
          <p className="mt-1 whitespace-pre-line text-sm text-[var(--color-ink)]">{notes}</p>
        </div>
      )}
    </div>
  );
}
