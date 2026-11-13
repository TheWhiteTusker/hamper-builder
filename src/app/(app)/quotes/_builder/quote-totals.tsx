"use client";

import { formatMoney, type QuotePricing } from "@/lib/pricing";
import { Field, Row, TextField } from "./fields";
import type { QuoteFields, SetField } from "./quote-state";

/** Notes and terms beside the order total, discount, charges and GST. */
export function QuoteTotals({
  f,
  set,
  totals,
  options,
  isCombined,
  canEdit,
}: {
  f: QuoteFields;
  set: SetField;
  totals: QuotePricing;
  /** Option-based only: each option's own totals. */
  options: { label: string; totals: QuotePricing }[];
  isCombined: boolean;
  canEdit: boolean;
}) {
  const off = !canEdit;
  const amount = (id: "orderDiscount" | "adj1" | "adj2" | "gstRate", label: string, className?: string) => (
    <TextField
      id={id}
      label={label}
      inputMode="decimal"
      className={`input-num ${className ?? ""}`}
      value={f[id]}
      onChange={(v) => set(id, v)}
      disabled={off}
    />
  );

  return (
    <section className="grid gap-4 lg:grid-cols-2">
      <div className="card p-4">
        <h2 className="text-sm font-semibold">Notes and terms</h2>
        <div className="mt-3 space-y-3">
          <Field label="Internal notes" htmlFor="notes">
            <textarea id="notes" rows={2} className="input mt-1" value={f.notes} onChange={(e) => set("notes", e.target.value)} disabled={off} />
          </Field>
          <Field label="Terms printed on the quotation" htmlFor="terms">
            <textarea id="terms" rows={5} className="input mt-1" value={f.terms} onChange={(e) => set("terms", e.target.value)} disabled={off} />
          </Field>
        </div>
      </div>

      <div className="card p-4">
        <h2 className="text-sm font-semibold">{isCombined ? "Order total" : "Option totals"}</h2>

        {!isCombined ? (
          <>
            <p className="mt-1 text-xs text-[var(--color-muted)]">
              Each option is totalled on its own; the client picks one. Order discount and charges apply to combined
              orders only.
            </p>
            <div className="mt-3">{amount("gstRate", "GST %", "max-w-[120px]")}</div>
            <dl className="mt-4 space-y-1.5 border-t border-[var(--color-line)] pt-3 text-sm">
              {options.map((o) => (
                <Row key={o.label} label={`${o.label} (incl. GST)`} value={formatMoney(o.totals.grandTotal)} strong />
              ))}
            </dl>
          </>
        ) : (
          <>
            <dl className="mt-3 space-y-1.5 text-sm">
              <Row label="Subtotal" value={formatMoney(totals.subtotal)} />
            </dl>

            <div className="mt-3 grid gap-3 sm:grid-cols-3">
              {amount("orderDiscount", "Order discount")}
              {amount("adj1", "Packaging / freight")}
              {amount("adj2", "Other charges")}
            </div>

            <dl className="mt-4 space-y-1.5 text-sm">
              <Row label="Taxable value" value={formatMoney(totals.taxableValue)} />
            </dl>

            <div className="mt-3">{amount("gstRate", "GST %", "max-w-[120px]")}</div>

            <dl className="mt-4 space-y-1.5 border-t border-[var(--color-line)] pt-3 text-sm">
              <Row label="GST" value={formatMoney(totals.gstAmount)} />
              <Row label="Grand total" value={formatMoney(totals.grandTotal)} strong />
            </dl>
          </>
        )}
      </div>
    </section>
  );
}
