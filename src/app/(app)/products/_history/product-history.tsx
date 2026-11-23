import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import type { Category } from "@/lib/types";
import { HISTORY_SELECT, HistoryList, type HistoryEntry } from "./history-list";

/** "History" card for one product: its latest `shown` changes, and a link to the rest. */
export async function ProductHistory({
  productId,
  categories,
  shown = 10,
}: {
  productId: string;
  categories: Category[];
  shown?: number;
}) {
  const { supabase } = await requireRole("admin");
  const { data, count } = await supabase
    .from("product_history")
    .select(HISTORY_SELECT, { count: "exact" })
    .eq("product_id", productId)
    .order("changed_at", { ascending: false })
    .limit(shown)
    .overrideTypes<HistoryEntry[], { merge: false }>();
  const entries = data ?? [];

  return (
    <section className="card">
      <div className="flex items-center justify-between border-b border-line px-4 py-3">
        <h2 className="font-semibold">History</h2>
        {(count ?? 0) > shown && (
          <Link href={`/products/history?product=${productId}`} className="text-sm font-medium text-(--color-brand) hover:underline">
            See all {count} changes
          </Link>
        )}
      </div>
      {entries.length ? (
        <HistoryList entries={entries} categories={categories} />
      ) : (
        <p className="px-4 py-6 text-center text-sm text-(--color-muted)">
          No changes recorded yet.
        </p>
      )}
    </section>
  );
}
