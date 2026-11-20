import { notFound } from "next/navigation";
import { requireUser, canManage } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { QuoteBuilder } from "../quote-builder";
import { loadQuoteOptions } from "../data";
import type { Quote, QuoteItem } from "@/lib/types";

type Search = Promise<Record<string, string | string[] | undefined>>;

export default async function QuotePage({
  params,
  searchParams,
}: {
  params: Promise<{ docNo: string }>;
  searchParams: Search;
}) {
  const { docNo } = await params;
  const flags = await searchParams;

  const { supabase, profile } = await requireUser();

  const { data: quote } = await supabase
    .from("quotes")
    .select("*")
    .eq("doc_no", decodeURIComponent(docNo))
    .maybeSingle<Quote>();

  if (!quote) notFound();

  const [{ data: items }, options] = await Promise.all([
    supabase
      .from("quote_items")
      .select("*")
      .eq("quote_id", quote.id)
      .order("line_no")
      .overrideTypes<QuoteItem[], { merge: false }>(),
    loadQuoteOptions(supabase),
  ]);

  // Sales may edit their own quotations; managers and admins may edit any.
  const canEdit = canManage(profile.role) || quote.created_by === profile.id;

  return (
    <>
      <PageHeader
        title={quote.client_name}
        subtitle={`${quote.doc_type === "proforma_invoice" ? "Proforma Invoice" : "Quotation"} ${quote.doc_no}`}
      >
        <span className="badge">{quote.status}</span>
      </PageHeader>

      {flags.converted && (
        <p className="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">
          Created from quotation {String(flags.converted)}.
        </p>
      )}

      {!canEdit && (
        <p className="mb-4 rounded-md border border-(--color-line) bg-white px-3 py-2 text-sm text-(--color-muted)">
          This quotation belongs to someone else, so it is read only for you.
        </p>
      )}

      <QuoteBuilder
        quote={quote}
        items={items ?? []}
        hampers={options.hampers}
        products={options.products}
        clients={options.clients}
        packagingCategories={options.packagingCategories}
        settings={options.settings}
        canEdit={canEdit}
        canChangeStatus={canManage(profile.role)}
      />
    </>
  );
}
