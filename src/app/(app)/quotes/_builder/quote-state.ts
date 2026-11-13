import { COMBINED_ORDER, num, optionKey, round2 } from "@/lib/pricing";
import type { ContentItem } from "@/components/hamper-contents";
import type { Quote, QuoteItem, Settings } from "@/lib/types";

export type HamperOption = {
  id: string;
  code: string;
  name: string;
  final_catalogue_sp: number | null;
  items: ContentItem[];
};

export type ProductOption = {
  id: string;
  code: string;
  name: string;
  default_sp: number;
};

/** A quote line as typed: numbers stay strings until saved. */
export type Line = {
  key: string;
  option_label: string;
  hamper_id: string | null;
  hamper_code: string;
  hamper_name: string;
  qty: string;
  catalogue_price: string;
  discount_pct: string;
  detail_mode: string;
  packaging_treatment: string;
};

/** Everything on the quote besides its lines. Percentages are whole numbers here. */
export type QuoteFields = {
  docType: Quote["doc_type"];
  docDate: string;
  clientName: string;
  contactPerson: string;
  phone: string;
  email: string;
  billingAddress: string;
  gstin: string;
  occasion: string;
  structure: string;
  validity: string;
  status: string;
  followUp: string;
  notes: string;
  terms: string;
  orderDiscount: string;
  adj1: string;
  adj2: string;
  gstRate: string;
};

export type SetField = <K extends keyof QuoteFields>(key: K, value: QuoteFields[K]) => void;

export const initialFields = (quote: Quote | undefined, settings: Settings): QuoteFields => ({
  docType: quote?.doc_type ?? "quotation",
  docDate: quote?.doc_date ?? new Date().toISOString().slice(0, 10),
  clientName: quote?.client_name ?? "",
  contactPerson: quote?.contact_person ?? "",
  phone: quote?.phone ?? "",
  email: quote?.email ?? "",
  billingAddress: quote?.billing_address ?? "",
  gstin: quote?.gstin ?? "",
  occasion: quote?.occasion ?? "",
  structure: quote?.quote_structure ?? COMBINED_ORDER,
  validity: quote?.validity ?? settings.default_validity,
  status: quote?.status ?? "Draft",
  followUp: quote?.follow_up_date ?? "",
  notes: quote?.notes ?? "",
  terms: quote?.terms ?? settings.quote_terms,
  orderDiscount: String(quote?.order_discount ?? 0),
  adj1: String(quote?.adj1 ?? 0),
  adj2: String(quote?.adj2 ?? 0),
  gstRate: String(round2((quote?.gst_rate ?? settings.gst_rate) * 100)),
});

export const toLine = (item: QuoteItem): Line => ({
  key: crypto.randomUUID(),
  option_label: item.option_label ?? "",
  hamper_id: item.hamper_id,
  hamper_code: item.hamper_code ?? "",
  hamper_name: item.hamper_name ?? "",
  qty: String(item.qty),
  catalogue_price: String(item.catalogue_price),
  discount_pct: String(round2(item.discount_pct * 100)),
  detail_mode: item.detail_mode ?? "",
  packaging_treatment: item.packaging_treatment ?? "",
});

export const lineFromHamper = (hamper: HamperOption, option: string, settings: Settings): Line => ({
  key: crypto.randomUUID(),
  option_label: option,
  hamper_id: hamper.id,
  hamper_code: hamper.code,
  hamper_name: hamper.name,
  qty: "1",
  catalogue_price: String(hamper.final_catalogue_sp ?? 0),
  discount_pct: "0",
  detail_mode: settings.default_detail_mode,
  packaging_treatment: settings.default_packaging_treatment,
});

/**
 * A product quoted on its own. It rides in the same hamper_code/hamper_name
 * columns with no hamper_id, so saving, converting and printing need no changes.
 */
export const lineFromProduct = (product: ProductOption, option: string): Line => ({
  key: crypto.randomUUID(),
  option_label: option,
  hamper_id: null,
  hamper_code: product.code,
  hamper_name: product.name,
  qty: "1",
  catalogue_price: String(product.default_sp ?? 0),
  discount_pct: "0",
  detail_mode: "",
  packaging_treatment: "",
});

/** The first "Option N" not already taken. */
export function nextOptionLabel(taken: string[]) {
  let n = taken.length + 1;
  while (taken.includes(`Option ${n}`)) n++;
  return `Option ${n}`;
}

/** A line as priceQuote()/priceQuoteLine() take it. */
export const pricedLine = (l: Line) => ({
  qty: num(l.qty),
  cataloguePrice: num(l.catalogue_price),
  discountPct: num(l.discount_pct) / 100,
});

/** The JSON the saveQuote action receives. */
export const quotePayload = (f: QuoteFields, lines: Line[], quote?: Quote) =>
  JSON.stringify({
    id: quote?.id ?? null,
    doc_type: f.docType,
    doc_date: f.docDate,
    client_name: f.clientName,
    contact_person: f.contactPerson || null,
    phone: f.phone || null,
    email: f.email || null,
    billing_address: f.billingAddress || null,
    gstin: f.gstin || null,
    occasion: f.occasion || null,
    quote_structure: f.structure,
    validity: f.validity || null,
    status: f.status,
    order_discount: num(f.orderDiscount),
    adj1: num(f.adj1),
    adj2: num(f.adj2),
    gst_rate: num(f.gstRate) / 100,
    notes: f.notes || null,
    terms: f.terms || null,
    follow_up_date: f.followUp || null,
    linked_doc_no: quote?.linked_doc_no ?? null,
    lines: lines.map((l) => ({
      // A combined order is one set of confirmed items: no options.
      option_label: f.structure === COMBINED_ORDER ? null : optionKey(l.option_label),
      hamper_id: l.hamper_id,
      hamper_code: l.hamper_code || null,
      hamper_name: l.hamper_name || null,
      qty: num(l.qty),
      catalogue_price: num(l.catalogue_price),
      discount_pct: num(l.discount_pct) / 100,
      detail_mode: l.detail_mode || null,
      packaging_treatment: l.packaging_treatment || null,
    })),
  });
