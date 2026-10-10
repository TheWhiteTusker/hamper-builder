import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireUser, canManage } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { LoadMore } from "@/components/load-more";
import { pageLimit } from "@/lib/paging";
import { ClientForm } from "./client-form";
import type { Client } from "@/lib/types";

type Search = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

export default async function ClientsPage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const q = one(params.q).trim();
  const editId = one(params.edit);
  const limit = pageLimit(one(params.limit));

  const { supabase, profile } = await requireUser();

  let query = supabase.from("clients").select("*", { count: "exact" }).order("name");
  if (q) query = query.or(`name.ilike.%${q}%,gstin.ilike.%${q}%`);

  const { data, count } = await query.range(0, limit - 1).returns<Client[]>();
  const rows = data ?? [];
  const total = count ?? rows.length;

  // The client being edited may sit beyond the rows loaded so far.
  const editing = editId
    ? (rows.find((c) => c.id === editId) ??
      (await supabase.from("clients").select("*").eq("id", editId).maybeSingle<Client>()).data ??
      undefined)
    : undefined;

  return (
    <>
      <PageHeader
        title="Clients"
        subtitle={`${total} client${total === 1 ? "" : "s"} ·picked on quotations and proforma invoices`}
      />

      <div className="grid items-start gap-4 lg:grid-cols-[400px_1fr]">
        <ClientForm
          key={editing?.id ?? "new"}
          client={editing}
          canDelete={canManage(profile.role)}
        />

        <div>
          <form className="card mb-4 flex items-end gap-3 p-3">
            <div className="flex-1">
              <label className="label" htmlFor="q">
                Search
              </label>
              <input
                id="q"
                name="q"
                defaultValue={q}
                placeholder="Company name or GSTIN"
                className="input mt-1"
              />
            </div>
            <button type="submit" className="btn-secondary">
              Apply
            </button>
          </form>

          {rows.length === 0 ? (
            <p className="card px-4 py-8 text-center text-sm text-[var(--color-muted)]">
              {q ? "No clients match that search." : "No clients yet. Add one on the left."}
            </p>
          ) : (
            <div className="card overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Company</TableHead>
                    <TableHead>GSTIN</TableHead>
                    <TableHead>Contact</TableHead>
                    <TableHead></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {rows.map((c) => (
                    <TableRow key={c.id} className="align-top">
                      <TableCell>
                        <div className="font-medium">{c.name}</div>
                        {c.billing_address && (
                          <div className="whitespace-pre-line text-xs text-[var(--color-muted)]">
                            {c.billing_address}
                          </div>
                        )}
                      </TableCell>
                      <TableCell className="font-mono whitespace-nowrap">{c.gstin ?? "—"}</TableCell>
                      <TableCell className="text-[var(--color-muted)]">
                        {[c.contact_person, c.phone, c.email].filter(Boolean).join(" · ") || "—"}
                      </TableCell>
                      <TableCell className="num">
                        <Link
                          href={`/clients?edit=${c.id}`}
                          className="font-medium text-[var(--color-brand)] hover:underline"
                        >
                          Edit
                        </Link>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}

          <LoadMore shown={rows.length} total={total} />
        </div>
      </div>
    </>
  );
}
