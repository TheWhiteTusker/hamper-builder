import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { PageHeader, EmptyState } from "@/components/ui";
import { LoadMore } from "@/components/load-more";
import { pageLimit } from "@/lib/paging";
import type { Category } from "@/lib/types";
import { FilterForm } from "../_list/filter-form";
import { HISTORY_SELECT, HistoryList, type HistoryEntry } from "../_history/history-list";

type Search = Promise<Record<string, string | string[] | undefined>>;

const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

const ACTIONS = [
  ["created", "Created"],
  ["edited", "Edited"],
  ["deleted", "Moved to Bin"],
  ["restored", "Restored"],
  ["purged", "Deleted permanently"],
] as const;

/** Every product created, edited, binned or restored, newest first. Admins only. */
export default async function ProductHistoryPage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const q = one(params.q).trim();
  const person = one(params.person);
  const action = one(params.action);
  const product = one(params.product);
  const limit = pageLimit(one(params.limit));

  const { supabase } = await requireRole("admin");

  let query = supabase
    .from("product_history")
    .select(HISTORY_SELECT, { count: "exact" })
    .order("changed_at", { ascending: false });
  if (q) query = query.or(`product_code.ilike.%${q}%,product_name.ilike.%${q}%`);
  if (person) query = query.eq("changed_by", person);
  if (action) query = query.eq("action", action);
  if (product) query = query.eq("product_id", product);

  const [{ data, count }, { data: people }, { data: categories }, { data: products }] = await Promise.all([
    query.range(0, limit - 1).overrideTypes<HistoryEntry[], { merge: false }>(),
    supabase.from("profiles").select("id, full_name").order("full_name"),
    supabase.from("categories").select("*").overrideTypes<Category[], { merge: false }>(),
    // Binned ones too: their history is still here.
    supabase.from("products").select("code, name").order("code"),
  ]);
  const entries = data ?? [];
  const total = count ?? entries.length;

  return (
    <>
      <PageHeader
        title="Product history"
        subtitle={product && entries[0] ? `${entries[0].product_code} · ${entries[0].product_name}` : `${total} change${total === 1 ? "" : "s"}`}
      >
        <Link href="/products" className="btn-secondary">
          Products
        </Link>
      </PageHeader>

      <FilterForm className="card mb-4 flex flex-wrap items-end gap-3 p-3">
        {product && <input type="hidden" name="product" value={product} />}
        <div className="min-w-55 flex-1">
          <label className="label" htmlFor="q">
            Find a product
          </label>
          {/* Suggests products as you type; picking one shows just its history. */}
          <input
            id="q"
            name="q"
            type="search"
            list="history-products"
            defaultValue={q}
            placeholder="Type a code or name, e.g. HO/0001/WL"
            autoComplete="off"
            className="input mt-1"
          />
          <datalist id="history-products">
            {(products ?? []).map((p) => (
              <option key={p.code} value={p.code}>
                {p.name}
              </option>
            ))}
          </datalist>
        </div>
        <div className="min-w-45">
          <label className="label" htmlFor="person">
            Changed by
          </label>
          <select id="person" name="person" defaultValue={person} className="select mt-1">
            <option value="">Anyone</option>
            {(people ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.full_name ?? "Unnamed"}
              </option>
            ))}
          </select>
        </div>
        <div className="min-w-45">
          <label className="label" htmlFor="action">
            Change
          </label>
          <select id="action" name="action" defaultValue={action} className="select mt-1">
            <option value="">Any change</option>
            {ACTIONS.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>
        {product && (
          <Link href="/products/history" className="pb-1.5 text-sm font-medium text-(--color-brand) hover:underline">
            All products
          </Link>
        )}
      </FilterForm>

      {entries.length === 0 ? (
        <EmptyState
          title={q || person || action || product ? "No changes match that search" : "No changes recorded yet"}
          hint="Try part of the code or name."
        />
      ) : (
        <div className="card list-scroll">
          <HistoryList entries={entries} categories={categories ?? []} showProduct />
          <LoadMore shown={entries.length} total={total} />
        </div>
      )}
    </>
  );
}
