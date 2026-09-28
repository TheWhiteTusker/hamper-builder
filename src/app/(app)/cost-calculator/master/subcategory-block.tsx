"use client";

import { useState } from "react";
import { formatMoney } from "@/lib/pricing";
import type { CostSubcategoryWithVarieties } from "@/lib/types";
import { deleteCostSubcategory, deleteCostVariety, renameCostSubcategory } from "./actions";
import { Chevron, type MasterCtx } from "./master-ui";

export function SubcategoryBlock({
  sub,
  categoryId,
  isMachine,
  stageCode,
  ctx,
}: {
  sub: CostSubcategoryWithVarieties;
  categoryId: string;
  isMachine: boolean;
  stageCode?: string;
  ctx: MasterCtx;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(sub.name);
  const open = ctx.isOpen(sub.id);
  const isBoughtOut = stageCode === "bought_out";

  function handleRename(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (!trimmed || trimmed === sub.name) {
      setIsEditing(false);
      setName(sub.name);
      return;
    }
    ctx.run(
      () => renameCostSubcategory(sub.id, trimmed),
      `Subcategory renamed to "${trimmed}".`,
      () => setIsEditing(false),
    );
  }

  function handleDelete() {
    if (!window.confirm(`Are you sure you want to delete subcategory "${sub.name}" and all its varieties?`)) return;
    ctx.run(() => deleteCostSubcategory(sub.id), `Subcategory "${sub.name}" deleted.`);
  }

  return (
    <div className="rounded-lg border border-slate-200 bg-white p-3 shadow-xs min-w-0 max-w-full">
      <div
        className={`flex flex-wrap items-center justify-between gap-2 ${open ? "border-b border-slate-100 pb-2 mb-2" : ""}`}
      >
        {isEditing ? (
          <form onSubmit={handleRename} className="flex min-w-0 flex-1 flex-wrap items-center gap-2 py-0.5">
            <span className="text-xs font-semibold text-(--color-muted)">Subcategory:</span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              placeholder="Subcategory name"
              className="input text-xs py-1 font-bold text-(--color-brand-dark) w-full sm:w-auto sm:max-w-xs"
            />
            <button
              type="submit"
              disabled={ctx.isPending || !name.trim()}
              className="btn-primary text-xs py-1 px-2.5"
            >
              {ctx.isPending ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              onClick={() => {
                setIsEditing(false);
                setName(sub.name);
              }}
              disabled={ctx.isPending}
              className="btn-secondary text-xs py-1 px-2.5"
            >
              Cancel
            </button>
          </form>
        ) : (
          <>
            <button
              type="button"
              onClick={() => ctx.toggle(sub.id)}
              aria-expanded={open}
              className="flex min-w-0 items-center gap-2 text-left"
            >
              <Chevron open={open} />
              <span className="font-semibold text-xs text-(--color-ink) truncate">
                Subcategory: <span className="text-(--color-brand-dark) font-bold">{sub.name}</span>
              </span>
              <span className="shrink-0 text-[11px] text-(--color-muted)">
                ({sub.varieties.length} {sub.varieties.length === 1 ? "variety" : "varieties"})
              </span>
            </button>

            <div className="flex shrink-0 flex-wrap items-center gap-1.5">
              <button
                type="button"
                onClick={() => {
                  ctx.expand(sub.id);
                  ctx.editVariety(sub.id, {
                    unit: isMachine ? "min" : isBoughtOut ? "piece" : "sq ft",
                    default_rate: isMachine ? 15 : isBoughtOut ? 100 : 50,
                    default_wastage_pct: isBoughtOut ? 0 : isMachine ? 5 : 10,
                  });
                }}
                className="rounded bg-emerald-50 px-2 py-0.5 text-xs font-semibold text-(--color-brand-dark) hover:bg-emerald-100 transition-colors"
              >
                + Add Variety
              </button>
              <button
                type="button"
                onClick={() => ctx.openTransferSubcategory(sub, "move", categoryId)}
                className="rounded bg-sky-50 px-2 py-0.5 text-xs font-semibold text-sky-700 hover:bg-sky-100 transition-colors"
                title="Move this subcategory to another category"
              >
                Move
              </button>
              <button
                type="button"
                onClick={() => ctx.openTransferSubcategory(sub, "duplicate", categoryId)}
                className="rounded bg-indigo-50 px-2 py-0.5 text-xs font-semibold text-indigo-700 hover:bg-indigo-100 transition-colors"
                title="Duplicate this subcategory and its varieties"
              >
                Duplicate
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsEditing(true);
                  setName(sub.name);
                }}
                className="rounded bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-700 hover:bg-slate-200 transition-colors"
                title="Rename Subcategory"
              >
                Rename
              </button>
              <button
                type="button"
                onClick={handleDelete}
                className="px-1 text-xs text-red-500 hover:text-red-700 transition-colors"
                title="Delete Subcategory"
              >
                Delete
              </button>
            </div>
          </>
        )}
      </div>

      {open && (
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
      )}
    </div>
  );
}
