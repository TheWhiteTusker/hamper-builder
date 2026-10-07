import Link from "@/components/link";
import { requireUser, canManage } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { PageHeader } from "@/components/ui";
import { LoadMore } from "@/components/load-more";
import { pageLimit } from "@/lib/paging";
import { HamperRow } from "./_list/hamper-row";
import { Range } from "./_list/range";
import type { HamperSummary } from "@/lib/types";

type Search = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

export default async function HampersPage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const q = one(params.q).trim();
  const collection = one(params.collection);
  const minItems = one(params.items).trim();
  const minCost = one(params.cost_min).trim();
  const maxCost = one(params.cost_max).trim();
  const minPrice = one(params.price_min).trim();
  const maxPrice = one(params.price_max).trim();
  const limit = pageLimit(one(params.limit));

  const { supabase, profile } = await requireUser();
  const manage = canManage(profile.role);

  let query = supabase.from("hamper_summary").select("*", { count: "exact" }).order("code");
  if (q) query = query.or(`name.ilike.%${q}%,code.ilike.%${q}%`);
  if (collection) query = query.eq("collection", collection);
  // Items is a floor; cost and catalogue price are ranges, either end optional.
  if (minItems) query = query.gte("number_of_items", Number(minItems));
  if (minCost) query = query.gte("total_cp", Number(minCost));
  if (maxCost) query = query.lte("total_cp", Number(maxCost));
  if (minPrice) query = query.gte("final_catalogue_sp", Number(minPrice));
  if (maxPrice) query = query.lte("final_catalogue_sp", Number(maxPrice));

  const [{ data: hampers, count }, settings] = await Promise.all([
    query.range(0, limit - 1).returns<HamperSummary[]>(),
    loadSettings(supabase),
  ]);

  const rows = hampers ?? [];
  const total = count ?? rows.length;
  const filtered = !!(
    q ||
    collection ||
    minItems ||
    minCost ||
    maxCost ||
    minPrice ||
    maxPrice
  );

  return (
    <>
      <PageHeader title="Hampers" subtitle={`${total} hamper${total === 1 ? "" : "s"}`}>
        {profile.role === "admin" && (
          <Link href="/admin/refresh-prices" className="btn-secondary">
            Refresh prices
          </Link>
        )}
        {manage && (
          <Link href="/hampers/new" className="btn-primary">
            New hamper
          </Link>
        )}
      </PageHeader>

      {one(params.deleted) === "1" && (
        <div className="mb-4 flex items-center justify-between rounded-lg border border-[var(--color-line)] bg-[#faf8ee] px-4 py-3 text-sm text-[var(--color-ink)] shadow-xs">
          <div className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-emerald-600" />
            <span>Hamper moved to Bin. It will remain in the Bin for 30 days and can be restored anytime.</span>
          </div>
          <Link href="/bin?tab=hampers" className="font-semibold text-[var(--color-brand)] underline hover:text-[var(--color-brand-dark)]">
            Open Bin →
          </Link>
        </div>
      )}

      {/* The form wraps the whole table so every filter can sit in the header
          cell of the column it filters. */}
      <form className="card overflow-x-auto">
        <table className="table min-w-[1200px]">
          <thead>
            <tr>
              <th className="w-14"></th>
              <th>Code</th>
              <th>Hamper</th>
              <th>Collection</th>
              <th>Status</th>
              <th className="num">Items</th>
              <th className="num">Cost</th>
              <th className="num">Catalogue price</th>
              <th className="num">Profit</th>
              <th className="num">Margin</th>
            </tr>

            <tr>
              <th className="pb-2"></th>
              <th colSpan={2} className="pb-2">
                <input
                  name="q"
                  aria-label="Search name or code"
                  defaultValue={q}
                  placeholder="Name or code"
                  className="input"
                />
              </th>
              <th className="pb-2">
                <input
                  name="collection"
                  aria-label="Filter by collection"
                  list="collection-options"
                  defaultValue={collection}
                  placeholder="Any"
                  className="input"
                />
                <datalist id="collection-options">
                  {settings.collections.map((c) => (
                    <option key={c} value={c} />
                  ))}
                </datalist>
              </th>
              <th className="pb-2"></th>
              <th className="pb-2">
                <input
                  name="items"
                  aria-label="Minimum items"
                  inputMode="decimal"
                  defaultValue={minItems}
                  placeholder="min"
                  className="input input-num"
                />
              </th>
              <th className="pb-2">
                <Range name="cost" label="cost" min={minCost} max={maxCost} />
              </th>
              <th className="pb-2">
                <Range name="price" label="catalogue price" min={minPrice} max={maxPrice} />
              </th>
              <th colSpan={2} className="pb-2">
                <div className="flex items-center justify-end gap-2">
                  {filtered && (
                    <Link href="/hampers" className="btn-secondary">
                      Clear
                    </Link>
                  )}
                  <button type="submit" className="btn-secondary">
                    Apply
                  </button>
                </div>
              </th>
            </tr>
          </thead>

          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={10} className="py-10 text-center text-sm text-[var(--color-muted)]">
                  {filtered ? "No hampers match that search." : "No hampers yet."}
                  {manage && !filtered && (
                    <Link href="/hampers/new" className="ml-2 underline">
                      Build the first one
                    </Link>
                  )}
                </td>
              </tr>
            ) : (
              rows.map((h) => <HamperRow key={h.id} h={h} />)
            )}
          </tbody>
        </table>
      </form>

      <LoadMore shown={rows.length} total={total} />
    </>
  );
}
