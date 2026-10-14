import { formatMoney, formatPct, round2 } from "@/lib/pricing";
import type { HamperSummary } from "@/lib/types";

const tone = (v: number | null) => (v === null ? "text-(--color-ink)" : v < 0 ? "text-red-700" : "text-emerald-700");

/** The builder's pricing strip as the product page's price boxes, read-only. */
export function HamperTotals({ h }: { h: HamperSummary }) {
  const boxes: [string, string, string][] = [
    ["Total cost", formatMoney(h.total_cp), "text-(--color-ink)"],
    ["Base SP", formatMoney(h.base_sp), "text-(--color-ink)"],
    ["Discount", `${round2(h.discount_pct * 100)}%`, "text-(--color-ink)"],
    ["SP after discount", formatMoney(h.sp_after_discount), "text-(--color-ink)"],
    ["Target SP", h.target_sp == null ? "—" : formatMoney(h.target_sp), "text-(--color-ink)"],
    ["Variance", h.variance == null ? "—" : formatMoney(h.variance), tone(h.variance)],
    ["Final catalogue SP", h.final_catalogue_sp == null ? "—" : formatMoney(h.final_catalogue_sp), "text-(--color-brand-dark)"],
    ["Final margin", h.final_margin == null ? "—" : formatPct(h.final_margin), tone(h.final_margin)],
  ];

  return (
    <div className="card border-2 border-(--color-brand)/20 bg-gradient-to-br from-white to-slate-50 p-5 shadow-md">
      <h3 className="mb-4 text-base font-bold text-(--color-ink)">Hamper pricing</h3>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {boxes.map(([label, value, color]) => (
          <div key={label} className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
            <span className="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">{label}</span>
            <div className={`mt-1 font-mono text-2xl font-black ${color}`}>{value}</div>
          </div>
        ))}
      </div>
      {h.gross_profit != null && (
        <p className="mt-3 text-sm text-(--color-muted)">
          Gross profit <span className={`font-mono font-semibold ${tone(h.gross_profit)}`}>{formatMoney(h.gross_profit)}</span>
        </p>
      )}
    </div>
  );
}
