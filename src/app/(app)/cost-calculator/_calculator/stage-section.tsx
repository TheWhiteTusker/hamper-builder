"use client";

import { withOverhead } from "@/lib/costing";
import { formatMoney, num } from "@/lib/pricing";
import type { CostStageWithHierarchy } from "@/lib/types";
import { OverheadInput, SectionHeader } from "./cells";
import { StageLineRow } from "./stage-line-row";
import type { CostLines } from "./use-cost-lines";

export function StageSection({
  stage,
  api,
  collapsed,
  onToggle,
  overhead,
  onOverhead,
  freeEntry,
}: {
  stage: CostStageWithHierarchy;
  api: CostLines;
  collapsed: boolean;
  onToggle: () => void;
  /** Overhead % as typed, added to this stage's subtotal. */
  overhead: string;
  onOverhead: (pct: string) => void;
  /** Bought-out items typed in and saved to the master (outsourced products). */
  freeEntry?: boolean;
}) {
  const stageLines = api.lines.filter((l) => l.stage_code === stage.code);
  const linesTotal = stageLines.reduce((acc, l) => acc + l.line_total, 0);
  const stageTotal = withOverhead(linesTotal, overhead);
  const isMachine = stage.code === "machine";
  const isBoughtOut = stage.code === "bought_out";
  // Stages with no categories configured (Miscellaneous) take a free-text
  // description instead of the Category -> Subcategory -> Variety selects.
  const hasCats = stage.categories.length > 0;
  const named = hasCats || freeEntry; // Category, Subcategory and Variety columns
  const masterRate = hasCats && !freeEntry;
  // Hierarchy, size, (duration), rate / unit, qty, wastage, total, actions.
  const colCount = (named ? 3 : 1) + 3 + (isMachine ? 1 : 0) + (masterRate ? 1 : 2) + 4;

  return (
    <div className="card overflow-hidden shadow-sm">
      <SectionHeader
        lead={
          <button
            type="button"
            onClick={onToggle}
            aria-expanded={!collapsed}
            title={collapsed ? `Expand ${stage.name}` : `Collapse ${stage.name}`}
            className="flex h-6 w-6 items-center justify-center rounded border border-(--color-border) bg-white text-sm font-bold leading-none text-(--color-brand-dark) shadow-xs transition-colors hover:bg-emerald-50"
          >
            {collapsed ? "+" : "−"}
          </button>
        }
        badge={
          <span className="flex h-7 w-7 items-center justify-center rounded-full bg-(--color-brand) text-xs font-bold text-white">
            {stage.sort_order}
          </span>
        }
        title={stage.name}
        hint={
          collapsed
            ? `(Collapsed — ${stageLines.length} line${stageLines.length === 1 ? "" : "s"}, click + to edit)`
            : isMachine
              ? "(Per minute or hour of machine time, or by size — as set in the master)"
              : isBoughtOut
                ? freeEntry
                  ? "(Type Category → Subcategory → Variety, rate & unit — new items are added to the Rates & Hierarchy Master on save)"
                  : hasCats
                  ? "(Category → Subcategory → Variety — Cost Price = Rate × Qty)"
                  : "(Free-text description, rate & quantity — Cost Price = Rate × Qty)"
                : hasCats
                  ? "(Category → Subcategory → Variety, dimensions & wastage)"
                  : "(Free-text description, rate & quantity)"
        }
        extra={
          <OverheadInput
            stageName={stage.name}
            value={overhead}
            added={num(overhead) ? formatMoney(stageTotal - linesTotal) : null}
            onChange={onOverhead}
          />
        }
        totalLabel="Stage Total:"
        total={formatMoney(stageTotal)}
      />

      {!collapsed && (
        <>
          <div className="overflow-x-auto p-2">
            <table className="w-full text-left text-xs">
              <thead>
                <tr className="border-b border-(--color-border) text-(--color-muted) font-semibold">
                  {named ? (
                    <>
                      <th className="p-2 min-w-32.5">Category</th>
                      <th className="p-2 min-w-32.5">Subcategory</th>
                      <th className="p-2 min-w-30">Variety</th>
                    </>
                  ) : (
                    <th className="p-2 min-w-65">Description</th>
                  )}
                  <th className="p-2 min-w-21.25">Length</th>
                  <th className="p-2 min-w-21.25">Breadth</th>
                  <th className="p-2 min-w-20">Dim Unit</th>
                  {isMachine && <th className="p-2 min-w-27.5">Duration (Mins)</th>}
                  {masterRate ? (
                    <th className="p-2 min-w-27.5">Rate / Unit</th>
                  ) : (
                    <>
                      <th className="p-2 min-w-21.25">Rate</th>
                      <th className="p-2 min-w-22.5">Unit</th>
                    </>
                  )}
                  <th className="p-2 min-w-16.25">Qty</th>
                  <th className="p-2 min-w-18.75">Wastage %</th>
                  <th className="p-2 text-right min-w-23.75">Total Cost</th>
                  <th className="p-2 text-center min-w-25">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-(--color-border)">
                {stageLines.length === 0 ? (
                  <tr>
                    <td colSpan={colCount} className="py-6 text-center text-sm text-(--color-muted)">
                      No lines added for {stage.name}. Click &ldquo;+ Add Line&rdquo; below to start.
                    </td>
                  </tr>
                ) : (
                  stageLines.map((line) => (
                    <StageLineRow key={line.tempKey} stage={stage} line={line} api={api} freeEntry={freeEntry} />
                  ))
                )}
              </tbody>
            </table>
          </div>

          <div className="border-t border-(--color-border) bg-slate-50/50 p-2 text-right">
            <button
              type="button"
              onClick={() => api.addToStage(stage.code)}
              className="btn-secondary text-xs py-1 px-3"
            >
              + Add {stage.name} Line
            </button>
          </div>
        </>
      )}
    </div>
  );
}
