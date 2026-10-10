import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { requireRole } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { formatMoney } from "@/lib/pricing";
import { ApplyButton } from "./apply-button";

type PriceDiff = {
  hamper_code: string;
  line_no: number;
  product_code: string | null;
  product_name: string | null;
  old_cp: number | null;
  new_cp: number | null;
  old_sp: number | null;
  new_sp: number | null;
  note: "changed" | "missing";
};

type Search = Promise<Record<string, string | string[] | undefined>>;

export default async function RefreshPricesPage({ searchParams }: { searchParams: Search }) {
  const flags = await searchParams;
  const { supabase } = await requireRole("admin");

  // Preview only. Nothing is written until the button below is used.
  const { data, error } = await supabase.rpc("refresh_hamper_prices", { p_apply: false });
  const diffs = (data ?? []) as PriceDiff[];

  const changed = diffs.filter((d) => d.note === "changed");
  const missing = diffs.filter((d) => d.note === "missing");
  const hampers = new Set(changed.map((d) => d.hamper_code));

  return (
    <>
      <PageHeader
        title="Refresh hamper prices"
        subtitle="Re-reads saved hampers against the current Product Master"
      />

      <p className="card mb-4 p-4 text-sm text-[var(--color-muted)]">
        Saved hampers keep their own copy of each product&rsquo;s cost and price, so changing
        the Product Master never moves them on its own. This is where you pull those copies
        up to date. Final catalogue prices are left exactly as approved, and quotations are
        not touched at all.
      </p>

      {flags.applied && (
        <p className="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">
          Updated {String(flags.applied)} hamper line(s).
        </p>
      )}

      {error && (
        <p role="alert" className="mb-4 text-sm text-red-700">
          {error.message}
        </p>
      )}

      {changed.length === 0 && missing.length === 0 ? (
        <div className="card p-8 text-center">
          <p className="font-medium">Everything is already up to date.</p>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            No saved hamper differs from the Product Master.
          </p>
        </div>
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center gap-3">
            <p className="text-sm">
              <strong>{changed.length}</strong> line(s) across <strong>{hampers.size}</strong>{" "}
              hamper(s) would change.
              {missing.length > 0 && (
                <> {missing.length} line(s) reference a product that no longer exists.</>
              )}
            </p>
            {changed.length > 0 && <ApplyButton count={changed.length} />}
          </div>

          <div className="card overflow-x-auto">
            <Table className="min-w-[900px]">
              <TableHeader>
                <TableRow>
                  <TableHead>Hamper</TableHead>
                  <TableHead>Line</TableHead>
                  <TableHead>Product</TableHead>
                  <TableHead className="num">Cost now</TableHead>
                  <TableHead className="num">New cost</TableHead>
                  <TableHead className="num">Price now</TableHead>
                  <TableHead className="num">New price</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {diffs.map((d) => (
                  <TableRow key={`${d.hamper_code}-${d.line_no}`}>
                    <TableCell className="font-mono whitespace-nowrap">{d.hamper_code}</TableCell>
                    <TableCell className="text-[var(--color-muted)]">{d.line_no}</TableCell>
                    <TableCell>
                      {d.product_name}
                      <span className="ml-2 font-mono text-xs text-[var(--color-muted)]">
                        {d.product_code}
                      </span>
                    </TableCell>
                    <TableCell className="num text-[var(--color-muted)]">{formatMoney(d.old_cp)}</TableCell>
                    <TableCell className="num font-medium">
                      {d.note === "missing" ? "—" : formatMoney(d.new_cp)}
                    </TableCell>
                    <TableCell className="num text-[var(--color-muted)]">{formatMoney(d.old_sp)}</TableCell>
                    <TableCell className="num font-medium">
                      {d.note === "missing" ? "—" : formatMoney(d.new_sp)}
                    </TableCell>
                    <TableCell>
                      {d.note === "missing" && (
                        <span className="badge border-amber-300 bg-amber-50 text-amber-900">
                          not in Product Master
                        </span>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </>
      )}
    </>
  );
}
