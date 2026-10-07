import Link from "@/components/link";
import { requireUser, isAdmin } from "@/lib/supabase/server";
import { PageHeader, EmptyState } from "@/components/ui";
import { LoadMore } from "@/components/load-more";
import { pageLimit } from "@/lib/paging";
import type { Category, ProductWithCategory } from "@/lib/types";
import { ProductRow } from "./_list/product-row";
import { FilterForm } from "./_list/filter-form";

type Search = Promise<Record<string, string | string[] | undefined>>;

const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

export default async function ProductsPage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const q = one(params.q).trim();
  const category = one(params.category);
  const showInactive = one(params.inactive) === "1";
  const limit = pageLimit(one(params.limit));

  const { supabase, profile } = await requireUser();
  const admin = isAdmin(profile.role);

  let query = supabase
    .from("products")
    .select("*, categories(name, counts_as_item)", { count: "exact" })
    .is("deleted_at", null)
    .order("code");

  if (q) query = query.or(`name.ilike.%${q}%,code.ilike.%${q}%,source.ilike.%${q}%`);
  if (category) query = query.eq("category_id", category);
  if (!showInactive) query = query.eq("is_active", true);

  const [{ data: products, count }, { data: categories }] = await Promise.all([
    query.range(0, limit - 1).returns<ProductWithCategory[]>(),
    supabase.from("categories").select("*").order("sort_order").order("name").returns<Category[]>(),
  ]);

  const rows = products ?? [];
  const total = count ?? rows.length;

  return (
    <>
      <PageHeader
        title="Product Master"
        subtitle={`${total} product${total === 1 ? "" : "s"}${showInactive ? " including inactive" : ""}`}
      >
        <a href="/api/products/export" className="btn-secondary">
          Export CSV
        </a>
        {admin && (
          <>
            <Link href="/admin/import" className="btn-secondary">
              Import
            </Link>
            <Link href="/products/new" className="btn-primary">
              New product
            </Link>
          </>
        )}
      </PageHeader>

      {one(params.deleted) === "1" && (
        <div className="mb-4 flex items-center justify-between rounded-lg border border-line bg-paper px-4 py-3 text-sm text-(--color-ink) shadow-xs">
          <div className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-emerald-600" />
            <span>Product moved to Bin. It will remain in the Bin for 30 days and can be restored anytime.</span>
          </div>
          <Link href="/bin?tab=products" className="font-semibold text-(--color-brand) underline hover:text-(--color-brand-dark)">
            Open Bin →
          </Link>
        </div>
      )}

      <FilterForm className="card mb-4 flex flex-wrap items-end gap-3 p-3">
        <div className="min-w-55 flex-1">
          <label className="label" htmlFor="q">
            Search
          </label>
          <input
            id="q"
            name="q"
            defaultValue={q}
            placeholder="Name, code or source"
            className="input mt-1"
          />
        </div>

        <div className="min-w-45">
          <label className="label" htmlFor="category">
            Category
          </label>
          <select id="category" name="category" defaultValue={category} className="select mt-1">
            <option value="">All categories</option>
            {(categories ?? []).map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </div>

        <label className="flex items-center gap-2 pb-1.5 text-sm">
          <input type="checkbox" name="inactive" value="1" defaultChecked={showInactive} />
          Show inactive
        </label>
      </FilterForm>

      {rows.length === 0 ? (
        <EmptyState
          title={q || category ? "No products match that search" : "No products yet"}
          hint={
            admin
              ? "Import your Product Master sheet, or add products one at a time."
              : "An admin needs to add products before hampers can be built."
          }
          actionHref={admin ? "/admin/import" : undefined}
          actionLabel={admin ? "Import from spreadsheet" : undefined}
        />
      ) : (
        <div className="card overflow-x-auto">
          <table className="table">
            <thead>
              <tr>
                <th className="w-12 text-center">Photo</th>
                <th>Code</th>
                <th>Product</th>
                <th>Category</th>
                <th>Source</th>
                <th className="num">Cost</th>
                <th className="num">Markup</th>
                <th className="num">Selling price</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => (
                <ProductRow key={p.id} p={p} admin={admin} />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <LoadMore shown={rows.length} total={total} />
    </>
  );
}
