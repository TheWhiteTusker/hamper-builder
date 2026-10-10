import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireUser, canManage } from "@/lib/supabase/server";
import { PageHeader, Stat, EmptyState } from "@/components/ui";
import { formatMoney } from "@/lib/pricing";
import type { HamperSummary, QuoteSummary } from "@/lib/types";

export default async function Dashboard() {
  const { supabase, profile } = await requireUser();
  const today = new Date().toISOString().slice(0, 10);

  const [hampers, quotes, products, followUps] = await Promise.all([
    supabase
      .from("hamper_summary")
      .select("*")
      .order("updated_at", { ascending: false })
      .limit(6)
      .returns<HamperSummary[]>(),
    supabase
      .from("quote_summary")
      .select("*")
      .order("updated_at", { ascending: false })
      .limit(6)
      .returns<QuoteSummary[]>(),
    supabase.from("products").select("id", { count: "exact", head: true }).eq("is_active", true).is("deleted_at", null),
    supabase
      .from("quotes")
      .select("doc_no, client_name, follow_up_date, status")
      .not("follow_up_date", "is", null)
      .lte("follow_up_date", today)
      .not("status", "in", '("Won","Lost")')
      .order("follow_up_date")
      .limit(5),
  ]);

  const openValue = (quotes.data ?? [])
    .filter((q) => !["Won", "Lost", "Expired"].includes(q.status))
    .reduce((sum, q) => sum + (q.grand_total ?? q.subtotal ?? 0), 0);

  const empty = !hampers.data?.length && !quotes.data?.length;

  return (
    <>
      <PageHeader
        title={`Welcome back${profile.full_name ? `, ${profile.full_name.split(" ")[0]}` : ""}`}
        subtitle="Hamper costing and client quotations"
      >
        {canManage(profile.role) && (
          <Link href="/hampers/new" className="btn-secondary">
            New hamper
          </Link>
        )}
        <Link href="/quotes/new" className="btn-primary">
          New quote
        </Link>
      </PageHeader>

      {empty ? (
        <EmptyState
          title="Nothing here yet"
          hint={
            profile.role === "admin"
              ? "Import your spreadsheet to bring across products, saved hampers and past quotations."
              : "Once an admin imports the product list you can start building quotes."
          }
          actionHref={profile.role === "admin" ? "/admin/import" : undefined}
          actionLabel={profile.role === "admin" ? "Import from spreadsheet" : undefined}
        />
      ) : (
        <>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <Stat label="Active products" value={products.count ?? 0} />
            <Stat label="Saved hampers" value={hampers.data?.length ?? 0} hint="most recent" />
            <Stat label="Open quotes" value={quotes.data?.filter((q) => !["Won", "Lost", "Expired"].includes(q.status)).length ?? 0} />
            <Stat label="Open value" value={formatMoney(openValue)} hint="recent quotes, incl. GST" />
          </div>

          {!!followUps.data?.length && (
            <section className="card mt-6 p-4">
              <h2 className="text-sm font-semibold">Follow-ups due</h2>
              <ul className="mt-2 divide-y divide-[var(--color-line)]">
                {followUps.data.map((q) => (
                  <li key={q.doc_no} className="flex items-center justify-between py-2 text-sm">
                    <Link href={`/quotes/${q.doc_no}`} className="hover:underline">
                      <span className="font-mono">{q.doc_no}</span>
                      <span className="ml-2 text-[var(--color-muted)]">{q.client_name}</span>
                    </Link>
                    <span className="text-[var(--color-muted)]">{q.follow_up_date}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <div className="mt-6 grid gap-4 lg:grid-cols-2">
            <RecentHampers rows={hampers.data ?? []} />
            <RecentQuotes rows={quotes.data ?? []} />
          </div>
        </>
      )}
    </>
  );
}

function RecentHampers({ rows }: { rows: HamperSummary[] }) {
  return (
    <section className="card overflow-hidden">
      <div className="flex items-center justify-between border-b border-[var(--color-line)] px-4 py-2.5">
        <h2 className="text-sm font-semibold">Recent hampers</h2>
        <Link href="/hampers" className="text-sm text-[var(--color-brand)] hover:underline">
          View all
        </Link>
      </div>
      {rows.length === 0 ? (
        <p className="px-4 py-6 text-sm text-[var(--color-muted)]">No hampers yet.</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Code</TableHead>
              <TableHead>Name</TableHead>
              <TableHead className="num">Cost</TableHead>
              <TableHead className="num">Price</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((h) => (
              <TableRow key={h.id}>
                <TableCell className="font-mono">
                  <Link href={`/hampers/${h.code}`} className="hover:underline">
                    {h.code}
                  </Link>
                </TableCell>
                <TableCell>{h.name}</TableCell>
                <TableCell className="num">{formatMoney(h.total_cp)}</TableCell>
                <TableCell className="num">{formatMoney(h.final_catalogue_sp)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </section>
  );
}

function RecentQuotes({ rows }: { rows: QuoteSummary[] }) {
  return (
    <section className="card overflow-hidden">
      <div className="flex items-center justify-between border-b border-[var(--color-line)] px-4 py-2.5">
        <h2 className="text-sm font-semibold">Recent quotes</h2>
        <Link href="/quotes" className="text-sm text-[var(--color-brand)] hover:underline">
          View all
        </Link>
      </div>
      {rows.length === 0 ? (
        <p className="px-4 py-6 text-sm text-[var(--color-muted)]">No quotations yet.</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Document</TableHead>
              <TableHead>Client</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="num">Total</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((q) => (
              <TableRow key={q.id}>
                <TableCell className="font-mono">
                  <Link href={`/quotes/${q.doc_no}`} className="hover:underline">
                    {q.doc_no}
                  </Link>
                </TableCell>
                <TableCell>{q.client_name}</TableCell>
                <TableCell>
                  <span className="badge">{q.status}</span>
                </TableCell>
                <TableCell className="num">{formatMoney(q.grand_total ?? q.subtotal)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </section>
  );
}
