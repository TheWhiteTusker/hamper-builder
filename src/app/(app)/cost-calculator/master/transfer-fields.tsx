"use client";

import type { CostStageWithHierarchy } from "@/lib/types";

export type TransferMode = "move" | "duplicate";

/** Move / Duplicate switch in the transfer dialog's header. */
export function ModeTabs({ mode, onChange }: { mode: TransferMode; onChange: (mode: TransferMode) => void }) {
  const tab = (m: TransferMode, label: string, active: string) => (
    <button
      type="button"
      onClick={() => onChange(m)}
      className={`rounded-md px-2.5 py-1 transition-colors ${
        mode === m ? `bg-white font-bold ${active} shadow-xs` : "text-slate-600 hover:text-slate-900"
      }`}
    >
      {label}
    </button>
  );
  return (
    <div className="inline-flex rounded-lg bg-slate-100 p-1 text-xs font-medium">
      {tab("move", "Move", "text-sky-700")}
      {tab("duplicate", "Duplicate", "text-indigo-700")}
    </div>
  );
}

/** Every category, grouped by stage. A move can't target the current one. */
export function CategorySelect({
  id,
  value,
  onChange,
  stages,
  currentCategoryId,
  mode,
}: {
  id: string;
  value: string;
  onChange: (categoryId: string) => void;
  stages: CostStageWithHierarchy[];
  currentCategoryId: string;
  mode: TransferMode;
}) {
  return (
    <select id={id} value={value} onChange={(e) => onChange(e.target.value)} required className="select mt-1 w-full text-sm">
      {stages.map((stage) => (
        <optgroup key={stage.id} label={`${stage.name} (${stage.categories.length})`}>
          {stage.categories.map((c) => (
            <option key={c.id} value={c.id} disabled={mode === "move" && c.id === currentCategoryId}>
              {c.name} {c.id === currentCategoryId ? "(Current Category)" : ""}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}
