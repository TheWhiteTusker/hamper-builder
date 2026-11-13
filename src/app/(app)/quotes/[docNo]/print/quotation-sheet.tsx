import { COMBINED_ORDER, formatMoney, formatPct, groupByOption, priceOption, type QuotePricing } from "@/lib/pricing";
import type { CompanySettings, HamperItem, QuoteItem, QuoteSummary } from "@/lib/types";
import { LinesTable } from "./lines-table";
import { QuotationTotals, TotalRow } from "./quotation-totals";

/** The quotation as printed: letterhead, client, lines, totals and terms. */
export function QuotationSheet({
  quote,
  lines,
  byHamper,
  packagingCategories,
  company,
  totals,
}: {
  quote: QuoteSummary;
  lines: QuoteItem[];
  byHamper: Map<string, HamperItem[]>;
  packagingCategories: string[];
  company: CompanySettings;
  totals: QuotePricing;
}) {
  const isCombined = quote.quote_structure === COMBINED_ORDER;
  const title = quote.doc_type === "proforma_invoice" ? "Proforma Invoice" : "Quotation";

  return (
    <article className="print-sheet card mx-auto max-w-[820px] p-10 text-[13px] leading-relaxed">
      {/* ---------------- letterhead ---------------- */}
      <header className="flex flex-wrap items-start justify-between gap-6 border-b border-[var(--color-line)] pb-5">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{company.name}</h1>
          {company.address && (
            <p className="mt-1 whitespace-pre-line text-[var(--color-muted)]">
              {company.address}
            </p>
          )}
          <p className="mt-1 text-[var(--color-muted)]">
            {[company.phone, company.email, company.website].filter(Boolean).join("  ·  ")}
          </p>
          {company.gstin && (
            <p className="mt-1 text-[var(--color-muted)]">GSTIN: {company.gstin}</p>
          )}
        </div>

        <div className="text-right">
          <h2 className="text-base font-semibold uppercase tracking-wide">{title}</h2>
          <p className="mt-1 font-mono text-sm">{quote.doc_no}</p>
          <p className="text-[var(--color-muted)]">{quote.doc_date}</p>
          {quote.validity && (
            <p className="text-[var(--color-muted)]">Valid for {quote.validity}</p>
          )}
          {quote.linked_doc_no && (
            <p className="text-[var(--color-muted)]">Against {quote.linked_doc_no}</p>
          )}
        </div>
      </header>

      {/* ---------------- client ---------------- */}
      <section className="mt-5 flex flex-wrap justify-between gap-6">
        <div>
          <div className="label">Prepared for</div>
          <p className="mt-1 font-medium">{quote.client_name}</p>
          {quote.contact_person && <p>{quote.contact_person}</p>}
          {quote.billing_address && (
            <p className="whitespace-pre-line text-[var(--color-muted)]">
              {quote.billing_address}
            </p>
          )}
          <p className="text-[var(--color-muted)]">
            {[quote.phone, quote.email].filter(Boolean).join("  ·  ")}
          </p>
          {quote.gstin && <p className="text-[var(--color-muted)]">GSTIN: {quote.gstin}</p>}
        </div>

        {quote.occasion && (
          <div className="text-right">
            <div className="label">Occasion</div>
            <p className="mt-1">{quote.occasion}</p>
          </div>
        )}
      </section>

      {/* ---------------- lines ---------------- */}
      {isCombined ? (
        <>
          <div className="mt-5">
            <LinesTable lines={lines} byHamper={byHamper} packagingCategories={packagingCategories} />
          </div>
          <QuotationTotals quote={quote} totals={totals} />
        </>
      ) : (
        <>
          <p className="mt-5 rounded-md bg-[var(--color-brand-soft)] px-3 py-2 text-[var(--color-brand-dark)]">
            Each option below is a complete order with its own total. Please let us know which
            option you would like to proceed with.
          </p>
          {groupByOption(lines).map((option) => {
            const t = priceOption(
              option.lines.map((l) => ({ qty: l.qty, cataloguePrice: l.catalogue_price, discountPct: l.discount_pct })),
              quote.gst_rate,
            );
            return (
              <section key={option.label} className="mt-6">
                <h3 className="text-sm font-semibold uppercase tracking-wide text-[var(--color-brand-dark)]">{option.label}</h3>
                <LinesTable lines={option.lines} byHamper={byHamper} packagingCategories={packagingCategories} />
                <dl className="avoid-break ml-auto mt-2 w-full max-w-[320px] space-y-1">
                  <TotalRow label="Subtotal" value={formatMoney(t.subtotal)} />
                  <TotalRow label={`GST @ ${formatPct(quote.gst_rate, 0)}`} value={formatMoney(t.gstAmount)} />
                  <div className="mt-1 border-t border-[var(--color-line)] pt-1">
                    <TotalRow label={`${option.label} total`} value={formatMoney(t.grandTotal)} strong />
                  </div>
                </dl>
              </section>
            );
          })}
        </>
      )}

      {/* ---------------- terms ---------------- */}
      {quote.terms && (
        <section className="avoid-break mt-8 border-t border-[var(--color-line)] pt-4">
          <div className="label">Terms</div>
          <p className="mt-1 whitespace-pre-line text-[var(--color-muted)]">{quote.terms}</p>
        </section>
      )}

      <footer className="mt-8 text-center text-xs text-[var(--color-muted)]">
        {company.name}
        {company.website ? `  ·  ${company.website}` : ""}
      </footer>
    </article>
  );
}
