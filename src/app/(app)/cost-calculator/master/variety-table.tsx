"use client";

import { formatMoney } from "@/lib/pricing";
import type { CostSubcategoryWithVarieties } from "@/lib/types";
import type { MasterCtx } from "./master-ui";
import { deleteCostVariety } from "./variety-actions";

/** A subcategory's varieties, with Edit and Delete on each. */
export function VarietyTable({ sub, ctx }: { sub: CostSubcategoryWithVarieties; ctx: MasterCtx }) {
  return (
    <div className="overflow-x-auto min-w-0 max-w-full">
      <table className="w-full min-w-125 text-left text-xs">
        <thead>
          <tr className="border-b border-slate-100 text-(--color-muted) font-medium">
            <th className="pb-1 font-semibold">Variety / Spec</th>
            <th className="pb-1 font-semibold">Default Rate</th>
            <th className="pb-1 font-semibold">Unit</th>
            <th className="pb-1 font-semibold">Default Wastage %</th>
            <th className="pb-1 font-semibold">Notes</th>
            <th className="pb-1 text-right font-semibold">Actions</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-50">
          {sub.varieties.length === 0 ? (
            <tr>
              <td colSpan={6} className="py-2 text-(--color-muted) italic">
                No varieties in {sub.name} yet. Click &ldquo;+ Add Variety&rdquo; above.
              </td>
            </tr>
          ) : (
            sub.varieties.map((v) => (
              <tr key={v.id} className="hover:bg-slate-50 transition-colors">
                <td className="py-1.5 font-semibold text-(--color-ink)">{v.name}</td>
                <td className="py-1.5 font-mono font-semibold">{formatMoney(v.default_rate)}</td>
                <td className="py-1.5 text-(--color-muted)">{v.unit}</td>
                <td className="py-1.5 text-(--color-muted)">{v.default_wastage_pct}%</td>
                <td className="py-1.5 text-(--color-muted)">{v.notes ?? "—"}</td>
                <td className="py-1.5 text-right">
                  <div className="flex items-center justify-end gap-2">
                    <button
                      type="button"
                      onClick={() => ctx.editVariety(sub.id, v)}
                      className="text-xs text-(--color-brand) hover:underline"
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        if (!window.confirm(`Are you sure you want to delete variety "${v.name}"?`)) return;
                        ctx.run(() => deleteCostVariety(v.id), `Variety "${v.name}" deleted.`);
                      }}
                      className="text-xs text-red-600 hover:underline"
                    >
                      Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
