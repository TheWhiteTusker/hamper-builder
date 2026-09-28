"use client";

import { COMMON_UNITS } from "@/lib/costing.ts";
import type { CostVariety } from "@/lib/types";

/** Modal for adding or editing one variety under a subcategory. */
export function VarietyDialog({
  subcategoryId,
  variety,
  isPending,
  onSave,
  onCancel,
}: {
  subcategoryId: string;
  variety: Partial<CostVariety>;
  isPending: boolean;
  onSave: (fd: FormData) => void;
  onCancel: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4">
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSave(new FormData(e.currentTarget));
        }}
        className="card max-w-md w-full p-5 bg-white shadow-2xl space-y-4"
      >
        <input type="hidden" name="subcategory_id" value={subcategoryId} />
        {variety.id && <input type="hidden" name="id" value={variety.id} />}

        <h3 className="text-base font-bold text-(--color-ink)">
          {variety.id ? "Edit Variety / Specification" : "New Variety / Specification"}
        </h3>

        <div>
          <label className="label" htmlFor="variety-name">
            Variety / Specification Name *
          </label>
          <input
            id="variety-name"
            name="name"
            defaultValue={variety.name ?? ""}
            placeholder="e.g. 8mm, 12mm, 10x2mm, Matt"
            required
            className="input mt-1 text-sm font-semibold"
          />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="label" htmlFor="variety-rate">
              Default Rate (₹) *
            </label>
            <input
              id="variety-rate"
              name="default_rate"
              type="number"
              step="any"
              defaultValue={variety.default_rate ?? 0}
              required
              className="input input-num mt-1 text-sm font-mono"
            />
          </div>

          <div>
            <label className="label" htmlFor="variety-unit">
              Unit *
            </label>
            <select
              id="variety-unit"
              name="unit"
              defaultValue={variety.unit ?? "sq ft"}
              required
              className="select mt-1 text-sm"
            >
              {COMMON_UNITS.map((u) => (
                <option key={u} value={u}>
                  {u}
                </option>
              ))}
              {variety.unit &&
                !COMMON_UNITS.includes(variety.unit as (typeof COMMON_UNITS)[number]) && (
                  <option value={variety.unit}>{variety.unit}</option>
                )}
            </select>
          </div>
        </div>

        {variety.id && (
          <p className="-mt-2 rounded-md bg-amber-50 px-2 py-1.5 text-[11px] text-amber-900">
            A new rate or unit reprices every product costed with this variety. Quotations and invoices keep their
            prices.
          </p>
        )}

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="label" htmlFor="variety-wastage">
              Default Wastage %
            </label>
            <input
              id="variety-wastage"
              name="default_wastage_pct"
              type="number"
              step="any"
              defaultValue={variety.default_wastage_pct ?? 10}
              className="input input-num mt-1 text-sm"
            />
          </div>

          <div>
            <label className="label" htmlFor="variety-sort">
              Sort Order
            </label>
            <input
              id="variety-sort"
              name="sort_order"
              type="number"
              defaultValue={variety.sort_order ?? 0}
              className="input input-num mt-1 text-sm"
            />
          </div>
        </div>

        <div>
          <label className="label" htmlFor="variety-notes">
            Notes / Specs
          </label>
          <input
            id="variety-notes"
            name="notes"
            defaultValue={variety.notes ?? ""}
            placeholder="e.g. Grade B/BB Russian Birch, 12mm thickness"
            className="input mt-1 text-sm"
          />
        </div>

        <div className="flex items-center justify-end gap-2 pt-3 border-t border-(--color-line)">
          <button type="button" onClick={onCancel} className="btn-secondary text-sm">
            Cancel
          </button>
          <button type="submit" disabled={isPending} className="btn-primary text-sm">
            {isPending ? "Saving…" : "Save Variety"}
          </button>
        </div>
      </form>
    </div>
  );
}
