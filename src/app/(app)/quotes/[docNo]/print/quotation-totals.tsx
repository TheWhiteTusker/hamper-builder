import { formatMoney, formatPct, type QuotePricing } from "@/lib/pricing";
import type { QuoteSummary } from "@/lib/types";

/** Subtotal down to grand total, for combined-order quotations. */
export function QuotationTotals({ quote, totals }: { quote: QuoteSummary; totals: QuotePricing }) {
  return (
    <section className="mt-5 flex justify-end">
      <dl className="w-full max-w-[320px] space-y-1">
        <TotalRow label="Subtotal" value={formatMoney(totals.subtotal)} />
        {quote.order_discount > 0 && (
          <TotalRow
            label="Order discount"
            value={`− ${formatMoney(quote.order_discount)}`}
          />
        )}
        {quote.adj1 > 0 && (
          <TotalRow label="Packaging / freight" value={formatMoney(quote.adj1)} />
        )}
        {quote.adj2 > 0 && (
          <TotalRow label="Other charges" value={formatMoney(quote.adj2)} />
        )}
        <TotalRow label="Taxable value" value={formatMoney(totals.taxableValue)} />
        <TotalRow
          label={`GST @ ${formatPct(quote.gst_rate, 0)}`}
          value={formatMoney(totals.gstAmount)}
        />
        <div className="mt-1 border-t border-[var(--color-line)] pt-1">
          <TotalRow label="Grand total" value={formatMoney(totals.grandTotal)} strong />
        </div>
      </dl>
    </section>
  );
}

export function TotalRow({
  label,
  value,
  strong,
}: {
  label: string;
  value: string;
  strong?: boolean;
}) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className={strong ? "font-semibold" : "text-[var(--color-muted)]"}>{label}</dt>
      <dd className={`tabular-nums ${strong ? "text-base font-semibold" : ""}`}>{value}</dd>
    </div>
  );
}
