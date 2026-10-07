import { notFound } from "next/navigation";
import Link from "@/components/link";
import { requireUser } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { priceQuote } from "@/lib/pricing";
import { PrintButton } from "./print-button";
import { ProformaSheet } from "./proforma-sheet";
import { QuotationSheet } from "./quotation-sheet";
import type { Category, HamperItem, QuoteItem, QuoteSummary } from "@/lib/types";

export default async function PrintQuotePage({
  params,
}: {
  params: Promise<{ docNo: string }>;
}) {
  const { docNo } = await params;
  const { supabase } = await requireUser();

  const { data: quote } = await supabase
    .from("quote_summary")
    .select("*")
    .eq("doc_no", decodeURIComponent(docNo))
    .maybeSingle<QuoteSummary>();

  if (!quote) notFound();

  const [{ data: lines }, settings, { data: categories }] = await Promise.all([
    supabase
      .from("quote_items")
      .select("*")
      .eq("quote_id", quote.id)
      .order("line_no")
      .returns<QuoteItem[]>(),
    loadSettings(supabase),
    supabase.from("categories").select("*").returns<Category[]>(),
  ]);

  const hamperIds = (lines ?? [])
    .map((l) => l.hamper_id)
    .filter((id): id is string => !!id);

  const { data: contents } = hamperIds.length
    ? await supabase
        .from("hamper_items")
        .select("*")
        .in("hamper_id", hamperIds)
        .order("line_no")
        .returns<HamperItem[]>()
    : { data: [] as HamperItem[] };

  const byHamper = new Map<string, HamperItem[]>();
  for (const item of contents ?? []) {
    const list = byHamper.get(item.hamper_id) ?? [];
    list.push(item);
    byHamper.set(item.hamper_id, list);
  }

  // Packaging-type categories are the ones Settings marks as not counting
  // toward "No. of Items" - the box, the filler, the ribbon.
  const packagingCategories = (categories ?? [])
    .filter((c) => !c.counts_as_item)
    .map((c) => c.name);

  const totals = priceQuote(
    (lines ?? []).map((l) => ({
      qty: l.qty,
      cataloguePrice: l.catalogue_price,
      discountPct: l.discount_pct,
    })),
    {
      quoteStructure: quote.quote_structure,
      orderDiscount: quote.order_discount,
      adj1: quote.adj1,
      adj2: quote.adj2,
      gstRate: quote.gst_rate,
    },
  );

  const company = settings.company;

  const toolbar = (
    <div className="no-print mb-4 flex flex-wrap items-center gap-2">
      <Link href={`/quotes/${encodeURIComponent(quote.doc_no)}`} className="btn-secondary">
        Back to quotation
      </Link>
      <PrintButton />
      <p className="text-sm text-[var(--color-muted)]">
        Print, then choose “Save as PDF” to send this to a client.
      </p>
    </div>
  );

  if (quote.doc_type === "proforma_invoice") {
    return (
      <>
        {toolbar}
        <ProformaSheet
          quote={quote}
          lines={lines ?? []}
          byHamper={byHamper}
          packagingCategories={packagingCategories}
          company={company}
        />
      </>
    );
  }
  return (
    <>
      {toolbar}
      <QuotationSheet
        quote={quote}
        lines={lines ?? []}
        byHamper={byHamper}
        packagingCategories={packagingCategories}
        company={company}
        totals={totals}
      />
    </>
  );
}
