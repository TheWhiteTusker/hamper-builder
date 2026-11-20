import Image from "next/image";
import { amountInWords, formatMoney, priceQuote, COMBINED_ORDER } from "@/lib/pricing";
import { HamperContents } from "@/components/hamper-contents";
import type { CompanySettings, HamperItem, QuoteItem, QuoteSummary } from "@/lib/types";
import { ProformaTotals, line } from "./proforma-totals";

/** Blank rows under the items, so a short invoice still reads as the ruled sheet. */
const MIN_ROWS = 8;

const cell = `border-b border-r ${line} px-2 py-1 last:border-r-0`;

/**
 * Proforma invoice in the ruled Palm Length LLP format: logo and title,
 * seller / bill-to, SR NO / DESCRIPTION / QTY / RATE / AMOUNT, GST split,
 * amount in words, bank details beside terms.
 */
export function ProformaSheet({
  quote,
  lines,
  byHamper,
  packagingCategories,
  company,
}: {
  quote: QuoteSummary;
  lines: QuoteItem[];
  byHamper: Map<string, HamperItem[]>;
  packagingCategories: string[];
  company: CompanySettings;
}) {
  // An invoice always has a total, even if it came from an option-based quote.
  const totals = priceQuote(
    lines.map((l) => ({
      qty: l.qty,
      cataloguePrice: l.catalogue_price,
      discountPct: l.discount_pct,
    })),
    {
      quoteStructure: COMBINED_ORDER,
      orderDiscount: quote.order_discount,
      adj1: quote.adj1,
      adj2: quote.adj2,
      gstRate: quote.gst_rate,
    },
  );
  const grand = totals.grandTotal ?? 0;

  const date = quote.doc_date.split("-").reverse().join("/");

  return (
    <article className="print-sheet card mx-auto max-w-205 p-8 text-[13px] leading-snug text-neutral-800">
      <div className={`border ${line}`}>
        {/* ---------------- logo + title ---------------- */}
        <header className={`flex items-end justify-between border-b ${line}`}>
          <Image
            src="/lattice-lane-logo-black.png"
            alt="Lattice Lane"
            width={1920}
            height={1080}
            priority
            className="h-36 w-auto"
          />
          <h1 className="px-3 pb-2 text-right text-4xl font-light uppercase leading-tight tracking-wide text-neutral-600">
            Proforma
            <br />
            Invoice
          </h1>
        </header>

        {/* ---------------- seller / bill to ---------------- */}
        <section className={`grid grid-cols-2 gap-4 border-b ${line} px-2 py-2`}>
          <div className="space-y-0.5">
            <p className="text-base">{company.legal_name || company.name}</p>
            {company.address && <p className="whitespace-pre-line">{company.address}</p>}
            {company.phone && <p>Phone No : {company.phone}</p>}
            {company.email && <p>Email ID : {company.email}</p>}
            {company.gstin && <p>GST No: {company.gstin}</p>}
          </div>

          <div className="space-y-0.5 text-right">
            <p>PI NO: {quote.doc_no}</p>
            <p>DATE: {date}</p>
            {quote.linked_doc_no && <p>AGAINST: {quote.linked_doc_no}</p>}
            <p className="pt-1">BILL TO,</p>
            <p className="font-medium uppercase">{quote.client_name}</p>
            {quote.contact_person && <p>Attn: {quote.contact_person}</p>}
            {quote.billing_address && (
              <p className="whitespace-pre-line">{quote.billing_address}</p>
            )}
            {quote.gstin && <p>GST No: {quote.gstin}</p>}
          </div>
        </section>

        {/* ---------------- items ---------------- */}
        <table className="w-full table-fixed border-collapse">
          <thead>
            <tr className="text-sm uppercase">
              <th className={`${cell} w-16 font-normal`}>Sr No</th>
              <th className={`${cell} font-normal`}>Description</th>
              <th className={`${cell} w-20 font-normal`}>Qty</th>
              <th className={`${cell} w-28 font-normal`}>Rate</th>
              <th className={`${cell} w-32 font-normal`}>Amount</th>
            </tr>
          </thead>
          <tbody>
            {lines.map((l, index) => (
              <tr key={l.id} className="align-top">
                <td className={`${cell} text-center`}>{index + 1}</td>
                <td className={cell}>
                  {l.hamper_name}
                  <HamperContents
                    className="text-xs"
                    items={l.hamper_id ? (byHamper.get(l.hamper_id) ?? []) : []}
                    detailMode={l.detail_mode}
                    packagingTreatment={l.packaging_treatment}
                    packagingCategories={packagingCategories}
                  />
                </td>
                <td className={`${cell} text-center tabular-nums`}>{l.qty}</td>
                <td className={`${cell} text-right tabular-nums`}>{formatMoney(l.final_rate)}</td>
                <td className={`${cell} text-right tabular-nums`}>{formatMoney(l.amount)}</td>
              </tr>
            ))}
            {Array.from({ length: Math.max(0, MIN_ROWS - lines.length) }, (_, i) => (
              <tr key={`blank-${i}`}>
                {Array.from({ length: 5 }, (_, j) => (
                  <td key={j} className={`${cell} h-6`} />
                ))}
              </tr>
            ))}
          </tbody>
        </table>

        <ProformaTotals quote={quote} company={company} totals={totals} />

        <div className={`flex border-b ${line}`}>
          <div className={`w-24 shrink-0 border-r ${line} px-2 py-1 uppercase`}>In Words</div>
          <div className="px-2 py-1">{amountInWords(grand)}</div>
        </div>

        {/* ---------------- bank + terms ---------------- */}
        <section className={`avoid-break grid grid-cols-[2fr_3fr] border-b ${line}`}>
          <div className={`border-r ${line}`}>
            <p className={`border-b ${line} px-2 py-1 text-sm uppercase`}>Bank Detail:</p>
            {[
              ["Name", company.bank_account_name || company.legal_name],
              ["Bank", company.bank_name],
              ["A/c No", company.bank_account_no],
              ["IFSC Code", company.bank_ifsc],
            ].map(([label, value]) => (
              <p key={label} className={`border-b ${line} px-2 py-1.5 text-sm last:border-b-0`}>
                {label}: {value}
              </p>
            ))}
          </div>

          <div>
            <p className={`border-b ${line} px-2 py-1 text-right text-sm uppercase`}>
              Terms &amp; Condition
            </p>
            {quote.terms && (
              <p className="whitespace-pre-line px-2 py-1 text-right text-[11px] leading-snug">
                {quote.terms}
              </p>
            )}
          </div>
        </section>

        <p className="px-3 py-2 text-right text-base uppercase text-neutral-600">
          Pleasure doing business with you.
        </p>
      </div>
    </article>
  );
}
