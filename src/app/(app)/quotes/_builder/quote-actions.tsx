"use client";

import { useActionState } from "react";
import Link from "@/components/link";
import type { Quote } from "@/lib/types";
import { convertToProforma, deleteQuote } from "../actions";

/**
 * Print, convert and delete for a saved quotation. An option-based quotation
 * converts one option, the one the client chose, into a combined-order invoice.
 */
export function QuoteActions({ quote, canEdit, options }: { quote: Quote; canEdit: boolean; options: string[] }) {
  const [delState, delAction, deleting] = useActionState(deleteQuote, {});
  const [convState, convAction, converting] = useActionState(convertToProforma, {});

  return (
    <>
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <Link href={`/quotes/${encodeURIComponent(quote.doc_no)}/print`} className="btn-secondary">
          Print / PDF
        </Link>

        {canEdit && quote.doc_type === "quotation" && (
          <form action={convAction} className="flex flex-wrap items-center gap-2">
            <input type="hidden" name="doc_no" value={quote.doc_no} />
            {options.length > 1 && (
              <select name="option" aria-label="Option the client chose" className="select" required defaultValue="">
                <option value="" disabled>
                  Client chose…
                </option>
                {options.map((o) => (
                  <option key={o} value={o}>
                    {o}
                  </option>
                ))}
              </select>
            )}
            {options.length === 1 && <input type="hidden" name="option" value={options[0]} />}
            <button type="submit" className="btn-secondary" disabled={converting}>
              {converting ? "Converting…" : "Convert to proforma invoice"}
            </button>
          </form>
        )}

        {canEdit && (
          <form action={delAction} className="ml-auto">
            <input type="hidden" name="id" value={quote.id} />
            <button type="submit" className="btn-danger" disabled={deleting}>
              {deleting ? "Deleting…" : "Delete quotation"}
            </button>
          </form>
        )}
      </div>

      {(delState.error || convState.error) && (
        <p role="alert" className="mt-2 text-sm text-red-700">
          {delState.error || convState.error}
        </p>
      )}
    </>
  );
}
