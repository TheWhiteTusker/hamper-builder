import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireUser } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { PageHeader, EmptyState } from "@/components/ui";
import { LoadMore } from "@/components/load-more";
import { pageLimit } from "@/lib/paging";
import { formatMoney } from "@/lib/pricing";
import type { QuoteSummary } from "@/lib/types";

type Search = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

const DOC_LABEL = { quotation: "Quotation", proforma_invoice: "Proforma Invoice" } as const;

export default async function QuotesPage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const q = one(params.q).trim();
  const status = one(params.status);
  const type = one(params.type);
  const limit = pageLimit(one(params.limit));

  const { supabase } = await requireUser();

  let query = supabase
    .from("quote_summary")
    .select("*", { count: "exact" })
    .order("doc_no", { ascending: false });
  if (q) query = query.or(`client_name.ilike.%${q}%,doc_no.ilike.%${q}%,occasion.ilike.%${q}%`);
  if (status) query = query.eq("status", status);
  if (type) query = query.eq("doc_type", type);

  const [{ data: quotes, count }, settings] = await Promise.all([
    query.range(0, limit - 1).returns<QuoteSummary[]>(),
    loadSettings(supabase),
  ]);

  const rows = quotes ?? [];
  const total = count ?? rows.length;

  return (
    <>
      <PageHeader
        title="Quotation Register"
        subtitle={`${total} document${total === 1 ? "" : "s"}`}
      >
        <Link href="/quotes/new" className="btn-primary">
          New quotation
        </Link>
      </PageHeader>

      <form className="card mb-4 flex flex-wrap items-end gap-3 p-3">
        <div className="min-w-[220px] flex-1">
          <label className="label" htmlFor="q">
            Search
          </label>
          <input
            id="q"
            name="q"
            defaultValue={q}
            placeholder="Client, document number or occasion"
            className="input mt-1"
          />
        </div>

        <div className="min-w-[150px]">
          <label className="label" htmlFor="type">
            Type
          </label>
          <select id="type" name="type" defaultValue={type} className="select mt-1">
            <option value="">Any type</option>
            <option value="quotation">Quotation</option>
            <option value="proforma_invoice">Proforma Invoice</option>
          </select>
        </div>

        <div className="min-w-[150px]">
          <label className="label" htmlFor="status">
            Status
          </label>
          <select id="status" name="status" defaultValue={status} className="select mt-1">
            <option value="">Any status</option>
            {settings.quote_statuses.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>

        <button type="submit" className="btn-secondary">
          Apply
        </button>
      </form>

      {rows.length === 0 ? (
        <EmptyState
          title={q || status || type ? "No documents match that search" : "No quotations yet"}
          hint="A quotation pulls saved hampers together for one client, then prints as a document."
          actionHref="/quotes/new"
          actionLabel="New quotation"
        />
      ) : (
        <div className="card overflow-x-auto">
          <Table className="min-w-[1050px]">
            <TableHeader>
              <TableRow>
                <TableHead>Document</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Date</TableHead>
                <TableHead>Client</TableHead>
                <TableHead>Occasion</TableHead>
                <TableHead className="num">Hampers</TableHead>
                <TableHead className="num">Qty</TableHead>
                <TableHead className="num">Total</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Follow up</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((quote) => (
                <TableRow key={quote.id}>
                  <TableCell className="font-mono whitespace-nowrap">
                    <Link
                      href={`/quotes/${encodeURIComponent(quote.doc_no)}`}
                      className="hover:underline"
                    >
                      {quote.doc_no}
                    </Link>
                    {quote.linked_doc_no && (
                      <div className="text-xs text-[var(--color-muted)]">
                        from {quote.linked_doc_no}
                      </div>
                    )}
                  </TableCell>
                  <TableCell className="whitespace-nowrap text-[var(--color-muted)]">
                    {DOC_LABEL[quote.doc_type]}
                  </TableCell>
                  <TableCell className="whitespace-nowrap">{quote.doc_date}</TableCell>
                  <TableCell>{quote.client_name}</TableCell>
                  <TableCell className="text-[var(--color-muted)]">{quote.occasion ?? "—"}</TableCell>
                  <TableCell className="num">{quote.line_count}</TableCell>
                  <TableCell className="num">{quote.total_qty}</TableCell>
                  <TableCell className="num">
                    {quote.grand_total == null ? (
                      <span className="text-[var(--color-muted)]" title="Option based quotation">
                        by option
                      </span>
                    ) : (
                      formatMoney(quote.grand_total)
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="badge">{quote.status}</span>
                  </TableCell>
                  <TableCell className="whitespace-nowrap text-[var(--color-muted)]">
                    {quote.follow_up_date ?? "—"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <LoadMore shown={rows.length} total={total} />
    </>
  );
}
