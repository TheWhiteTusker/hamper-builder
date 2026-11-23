"use client";

import { memo } from "react";
import { TableBody, TableCell, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { useLiveRows } from "@/components/use-live";
import { mergeQuote } from "@/lib/live-merge";
import { formatMoney } from "@/lib/pricing";
import type { QuoteSummary } from "@/lib/types";

const DOC_LABEL = { quotation: "Quotation", proforma_invoice: "Proforma Invoice" } as const;

/** The list's rows, patched the moment someone else changes a quote. */
export function QuoteRows({ rows: serverRows }: { rows: QuoteSummary[] }) {
  const rows = useLiveRows(serverRows, "quotes", mergeQuote);
  return (
    <TableBody>
      {rows.map((quote) => (
        <QuoteRow key={quote.id} quote={quote} />
      ))}
    </TableBody>
  );
}

/** Memo: a live change re-renders only its own row. */
const QuoteRow = memo(function QuoteRow({ quote }: { quote: QuoteSummary }) {
  return (
    <TableRow>
      <TableCell className="font-mono whitespace-nowrap">
        <Link href={`/quotes/${encodeURIComponent(quote.doc_no)}`} className="hover:underline">
          {quote.doc_no}
        </Link>
        {quote.linked_doc_no && <div className="text-xs text-(--color-muted)">from {quote.linked_doc_no}</div>}
      </TableCell>
      <TableCell className="whitespace-nowrap text-(--color-muted)">{DOC_LABEL[quote.doc_type]}</TableCell>
      <TableCell className="whitespace-nowrap">{quote.doc_date}</TableCell>
      <TableCell>{quote.client_name}</TableCell>
      <TableCell className="text-(--color-muted)">{quote.occasion ?? "—"}</TableCell>
      <TableCell className="num">{quote.line_count}</TableCell>
      <TableCell className="num">{quote.total_qty}</TableCell>
      <TableCell className="num">
        {quote.grand_total == null ? (
          <span className="text-(--color-muted)" title="Option based quotation">
            by option
          </span>
        ) : (
          formatMoney(quote.grand_total)
        )}
      </TableCell>
      <TableCell>
        <span className="badge">{quote.status}</span>
      </TableCell>
      <TableCell className="whitespace-nowrap text-(--color-muted)">{quote.follow_up_date ?? "—"}</TableCell>
    </TableRow>
  );
});
