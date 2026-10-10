import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { HamperContents } from "@/components/hamper-contents";
import { COMBINED_ORDER, formatMoney, type QuotePricing } from "@/lib/pricing";
import type { CompanySettings, HamperItem, QuoteItem, QuoteSummary } from "@/lib/types";
import { QuotationTotals } from "./quotation-totals";

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

      {!isCombined && (
        <p className="mt-5 rounded-md bg-[var(--color-brand-soft)] px-3 py-2 text-[var(--color-brand-dark)]">
          The options below are priced individually. Please select the one you would like
          to proceed with.
        </p>
      )}

      {/* ---------------- lines ---------------- */}
      <Table className="mt-5">
        <TableHeader>
          <TableRow>
            <TableHead className="w-8">#</TableHead>
            <TableHead>Item</TableHead>
            <TableHead className="num w-16">Qty</TableHead>
            <TableHead className="num w-24">Rate</TableHead>
            <TableHead className="num w-24">Amount</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {lines.map((line, index) => {
            const items = line.hamper_id ? (byHamper.get(line.hamper_id) ?? []) : [];

            return (
              <TableRow key={line.id} className="avoid-break align-top">
                <TableCell className="text-[var(--color-muted)]">
                  {line.option_label || index + 1}
                </TableCell>

                <TableCell>
                  <div className="font-medium">{line.hamper_name}</div>

                  <HamperContents
                    items={items}
                    detailMode={line.detail_mode}
                    packagingTreatment={line.packaging_treatment}
                    packagingCategories={packagingCategories}
                  />
                </TableCell>

                <TableCell className="num">{line.qty}</TableCell>

                <TableCell className="num">
                  {formatMoney(line.final_rate)}
                  {Number(line.discount_pct) > 0 && (
                    <div className="text-xs text-[var(--color-muted)] line-through">
                      {formatMoney(line.catalogue_price)}
                    </div>
                  )}
                </TableCell>

                <TableCell className="num font-medium">{formatMoney(line.amount)}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>

      {isCombined && <QuotationTotals quote={quote} totals={totals} />}

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
