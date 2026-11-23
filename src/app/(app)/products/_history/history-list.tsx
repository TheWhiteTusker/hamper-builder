import Link from "@/components/link";
import { formatMoney } from "@/lib/pricing";
import type { Category } from "@/lib/types";
import { productHref } from "../href";

/** A row of product_history (0026_product_history.sql), with who made it. */
export type HistoryEntry = {
  id: number;
  product_id: string;
  product_code: string;
  product_name: string;
  action: "created" | "edited" | "deleted" | "restored" | "purged";
  changes: Record<string, [unknown, unknown]>;
  changed_at: string;
  backfilled: boolean;
  profiles: { full_name: string | null } | null;
};

export const HISTORY_SELECT = "*, profiles(full_name)";

const VERB: Record<HistoryEntry["action"], string> = {
  created: "created",
  edited: "edited",
  deleted: "moved to Bin",
  restored: "restored from Bin",
  purged: "deleted permanently",
};

const LABEL: Record<string, string> = {
  code: "Code",
  name: "Name",
  category_id: "Category",
  source: "Source",
  cost_price: "Cost price",
  markup_pct: "Markup",
  default_sp: "Selling price",
  is_active: "Active",
  colors: "Colour",
  image_url: "Main photo",
  material_total: "Material cost",
  hardware_total: "Hardware cost",
  finishing_total: "Finishing cost",
  machine_total: "Machine cost",
  stage_overheads: "Overheads",
  notes: "Notes",
};

const MONEY = new Set(["cost_price", "default_sp", "material_total", "hardware_total", "finishing_total", "machine_total"]);
// Too long or opaque to show as from → to.
const CHANGED_ONLY = new Set(["image_url", "stage_overheads"]);

function show(field: string, v: unknown, categories: Category[]): string {
  if (v === null || v === undefined || v === "") return "—";
  if (MONEY.has(field)) return formatMoney(Number(v));
  if (field === "markup_pct") return `${v}%`;
  if (field === "is_active") return v ? "Yes" : "No";
  if (field === "category_id") return categories.find((c) => c.id === v)?.name ?? "a deleted category";
  if (Array.isArray(v)) return v.join(", ") || "—";
  const s = String(v);
  return s.length > 60 ? `${s.slice(0, 60)}…` : s;
}

const when = (iso: string) =>
  new Date(iso).toLocaleString("en-IN", { timeZone: "Asia/Kolkata", dateStyle: "medium", timeStyle: "short" });

/** Newest first. `showProduct` names the product on each line (the all-products page). */
export function HistoryList({
  entries,
  categories,
  showProduct = false,
}: {
  entries: HistoryEntry[];
  categories: Category[];
  showProduct?: boolean;
}) {
  return (
    <ul className="divide-y divide-line">
      {entries.map((e) => (
        <li key={e.id} className="flex flex-wrap items-start gap-x-4 gap-y-1 px-4 py-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm">
              <strong className="font-semibold">{e.profiles?.full_name ?? (e.backfilled ? "Someone" : "Unknown")}</strong>{" "}
              {e.backfilled && e.action === "edited" ? "last edited" : VERB[e.action]}
              {showProduct && (
                <>
                  {" "}
                  {e.action === "purged" ? (
                    <span className="font-mono">{e.product_code}</span>
                  ) : (
                    <Link href={productHref(e.product_code)} className="font-mono font-medium hover:underline">
                      {e.product_code}
                    </Link>
                  )}{" "}
                  <span className="text-(--color-muted)">{e.product_name}</span>
                </>
              )}
              {e.backfilled && (
                <span
                  className="badge ml-2"
                  title={
                    e.profiles
                      ? "From before history tracking: the person who first saved its costing."
                      : "From before history tracking: only the date is known."
                  }
                >
                  older record
                </span>
              )}
            </p>
            {Object.keys(e.changes).length > 0 && (
              <ul className="mt-1 space-y-0.5 text-xs text-(--color-muted)">
                {Object.entries(e.changes).map(([field, [from, to]]) => (
                  <li key={field}>
                    <span className="font-medium text-(--color-ink)">{LABEL[field] ?? field}</span>
                    {CHANGED_ONLY.has(field) ? (
                      " changed"
                    ) : (
                      <>
                        {": "}
                        {show(field, from, categories)} → {show(field, to, categories)}
                      </>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </div>
          <time dateTime={e.changed_at} className="text-xs whitespace-nowrap text-(--color-muted)">
            {when(e.changed_at)}
          </time>
        </li>
      ))}
    </ul>
  );
}
