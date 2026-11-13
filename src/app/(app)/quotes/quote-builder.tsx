"use client";

import { useActionState, useState } from "react";
import Link from "@/components/link";
import { saveQuote } from "./actions";
import { priceQuote, num, COMBINED_ORDER, groupByOption, optionKey, priceOption } from "@/lib/pricing";
import type { Client, Quote, QuoteItem, Settings } from "@/lib/types";
import { QuoteDetails } from "./_builder/quote-details";
import { QuoteActions } from "./_builder/quote-actions";
import { QuoteLines, type AddItem } from "./_builder/quote-lines";
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

  const [f, setFields] = useState<QuoteFields>(() => initialFields(quote, settings));
  const set: SetField = (key, value) => setFields((prev) => ({ ...prev, [key]: value }));

  const [lines, setLines] = useState<Line[]>(() => {
    if (items?.length) return items.map(toLine);
    // Arrived from a hamper page via "Add to a quote".
    const seed = initialHamperCode && hampers.find((h) => h.code === initialHamperCode);
    return seed ? [lineFromHamper(seed, "Option 1", settings)] : [];
  });

  const isCombined = f.structure === COMBINED_ORDER;
  const totals = priceQuote(lines.map(pricedLine), {
    quoteStructure: f.structure,
    orderDiscount: num(f.orderDiscount),
    adj1: num(f.adj1),
    adj2: num(f.adj2),
    gstRate: num(f.gstRate) / 100,
  });

  const options = isCombined
    ? []
    : groupByOption(lines).map((g) => ({ label: g.label, totals: priceOption(g.lines.map(pricedLine), num(f.gstRate) / 100) }));

  const addItem: AddItem = (kind, id, option) => {
    const hamper = kind === "hamper" && hampers.find((h) => h.id === id);
    const product = kind === "product" && products.find((p) => p.id === id);
    const line = hamper ? lineFromHamper(hamper, option, settings) : product ? lineFromProduct(product, option) : null;
    if (line) setLines((prev) => [...prev, line]);
  };
  const inOption = (label: string) => (l: Line) => optionKey(l.option_label) === label;

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
          gstPct={f.gstRate}
          onStructure={(s) => set("structure", s)}
          onAdd={addItem}
          onChange={(key, patch) => setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)))}
          onRemove={(key) => setLines((prev) => prev.filter((l) => l.key !== key))}
          onRenameOption={(from, to) => setLines((prev) => prev.map((l) => (inOption(from)(l) ? { ...l, option_label: to } : l)))}
          onRemoveOption={(label) => setLines((prev) => prev.filter((l) => !inOption(label)(l)))}
        />

        <QuoteTotals f={f} set={set} totals={totals} options={options} isCombined={isCombined} canEdit={canEdit} />

        {state.error && (
          <p role="alert" className="text-sm text-red-700">
            {state.error}
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
        <QuoteActions
          quote={quote}
          canEdit={canEdit}
          // Saved options, not unsaved edits: converting reads the stored quotation.
          options={quote.quote_structure === COMBINED_ORDER ? [] : groupByOption(items ?? []).map((g) => g.label)}
        />
      )}
    </>
  );
}
