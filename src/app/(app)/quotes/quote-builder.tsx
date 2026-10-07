"use client";

import { useActionState, useState } from "react";
import Link from "@/components/link";
import { saveQuote, deleteQuote, convertToProforma } from "./actions";
import { priceQuote, num, COMBINED_ORDER } from "@/lib/pricing";
import type { Client, Quote, QuoteItem, Settings } from "@/lib/types";
import { QuoteDetails } from "./_builder/quote-details";
import { QuoteLines } from "./_builder/quote-lines";
import {
  initialFields,
  lineFromHamper,
  lineFromProduct,
  pricedLine,
  quotePayload,
  toLine,
  type HamperOption,
  type Line,
  type ProductOption,
  type QuoteFields,
  type SetField,
} from "./_builder/quote-state";
import { QuoteTotals } from "./_builder/quote-totals";

export type { HamperOption, ProductOption };

export function QuoteBuilder({
  quote,
  items,
  hampers,
  products,
  clients,
  packagingCategories,
  settings,
  canEdit,
  canChangeStatus,
  initialHamperCode,
}: {
  quote?: Quote;
  items?: QuoteItem[];
  hampers: HamperOption[];
  products: ProductOption[];
  clients: Client[];
  packagingCategories: string[];
  settings: Settings;
  canEdit: boolean;
  canChangeStatus: boolean;
  initialHamperCode?: string;
}) {
  const [state, action, saving] = useActionState(saveQuote, {});
  const [delState, delAction, deleting] = useActionState(deleteQuote, {});
  const [convState, convAction, converting] = useActionState(convertToProforma, {});

  const [f, setFields] = useState<QuoteFields>(() => initialFields(quote, settings));
  const set: SetField = (key, value) => setFields((prev) => ({ ...prev, [key]: value }));

  const [lines, setLines] = useState<Line[]>(() => {
    if (items?.length) return items.map(toLine);
    // Arrived from a hamper page via "Add to a quote".
    const seed = initialHamperCode && hampers.find((h) => h.code === initialHamperCode);
    return seed ? [lineFromHamper(seed, 1, settings)] : [];
  });

  const isCombined = f.structure === COMBINED_ORDER;
  const totals = priceQuote(lines.map(pricedLine), {
    quoteStructure: f.structure,
    orderDiscount: num(f.orderDiscount),
    adj1: num(f.adj1),
    adj2: num(f.adj2),
    gstRate: num(f.gstRate) / 100,
  });

  function addHamper(hamperId: string) {
    const hamper = hampers.find((h) => h.id === hamperId);
    if (hamper) setLines((prev) => [...prev, lineFromHamper(hamper, prev.length + 1, settings)]);
  }

  function addProduct(productId: string) {
    const product = products.find((p) => p.id === productId);
    if (product) setLines((prev) => [...prev, lineFromProduct(product, prev.length + 1)]);
  }

  // Copies the client's details onto the document, so a sent quote keeps the
  // address it went out with even if the client record changes later.
  function pickClient(clientId: string) {
    const client = clients.find((c) => c.id === clientId);
    if (!client) return;
    setFields((prev) => ({
      ...prev,
      clientName: client.name,
      contactPerson: client.contact_person ?? "",
      phone: client.phone ?? "",
      email: client.email ?? "",
      billingAddress: client.billing_address ?? "",
      gstin: client.gstin ?? "",
    }));
  }

  // Quotations and proforma invoices have different default terms; swap them
  // only while the terms are still the untouched default.
  function changeDocType(next: Quote["doc_type"]) {
    const defaults = {
      quotation: settings.quote_terms,
      proforma_invoice: settings.pi_terms || settings.quote_terms,
    };
    setFields((prev) => ({
      ...prev,
      docType: next,
      terms: prev.terms === defaults[prev.docType] ? defaults[next] : prev.terms,
    }));
  }

  return (
    <>
      <form action={action} className="space-y-4">
        <input type="hidden" name="payload" value={quotePayload(f, lines, quote)} />

        <QuoteDetails
          f={f}
          set={set}
          clients={clients}
          settings={settings}
          canEdit={canEdit}
          canChangeStatus={canChangeStatus}
          onPickClient={pickClient}
          onDocType={changeDocType}
        />

        <QuoteLines
          lines={lines}
          structure={f.structure}
          isCombined={isCombined}
          hampers={hampers}
          products={products}
          settings={settings}
          packagingCategories={packagingCategories}
          canEdit={canEdit}
          onStructure={(s) => set("structure", s)}
          onAddHamper={addHamper}
          onAddProduct={addProduct}
          onChange={(key, patch) => setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)))}
          onRemove={(key) => setLines((prev) => prev.filter((l) => l.key !== key))}
        />

        <QuoteTotals f={f} set={set} totals={totals} isCombined={isCombined} canEdit={canEdit} />

        {(state.error || delState.error || convState.error) && (
          <p role="alert" className="text-sm text-red-700">
            {state.error || delState.error || convState.error}
          </p>
        )}

        {canEdit && (
          <div className="flex flex-wrap items-center gap-2">
            <button type="submit" className="btn-primary" disabled={saving}>
              {saving ? "Saving…" : quote ? "Save changes" : "Create quotation"}
            </button>
            <Link href="/quotes" className="btn-secondary">
              Cancel
            </Link>
          </div>
        )}
      </form>

      {quote && (
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <Link href={`/quotes/${encodeURIComponent(quote.doc_no)}/print`} className="btn-secondary">
            Print / PDF
          </Link>

          {canEdit && quote.doc_type === "quotation" && (
            <form action={convAction}>
              <input type="hidden" name="doc_no" value={quote.doc_no} />
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
      )}
    </>
  );
}
