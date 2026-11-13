"use client";

import { useState } from "react";
import type { CostStageWithHierarchy } from "@/lib/types";
import { saveCostCategory } from "./category-actions";
import { CategoryBlock } from "./category-block";
import { Chevron, InlineAdd, type MasterCtx } from "./master-ui";

export function StageCard({ stage, ctx }: { stage: CostStageWithHierarchy; ctx: MasterCtx }) {
  const [adding, setAdding] = useState(false);
  const open = ctx.isOpen(stage.id);
  const isMachine = stage.code === "machine";

  function handleAdd(name: string) {
    const fd = new FormData();
    fd.set("stage_id", stage.id);
    fd.set("name", name);
    ctx.run(() => saveCostCategory(fd), "Category added successfully!", () => setAdding(false));
  }

  // Only examples that fit the stage; the rest just show the levels.
  const example =
    stage.code === "bought_out"
      ? " (e.g. Ceramics → Jars → 200ml Amber)"
      : stage.code === "material"
        ? " (e.g. Woodbased → Birch → 8mm)"
        : "";
  const count = stage.categories.length;

  return (
    <div className="card">
      <div className="flex items-center justify-between gap-3 px-4 py-2.5">
        <button
          type="button"
          onClick={() => ctx.toggle(stage.id)}
          aria-expanded={open}
          className="flex min-w-0 flex-1 items-center gap-2 text-left"
        >
          <Chevron open={open} />
          <h3 className="text-base font-semibold text-(--color-ink)">{stage.name}</h3>
          <span className="text-xs text-(--color-muted)">
            {count} {count === 1 ? "category" : "categories"}
          </span>
        </button>

        <button
          type="button"
          onClick={() => {
            setAdding(true);
            ctx.expand(stage.id);
          }}
          className="btn-secondary px-3 py-1 text-xs"
        >
          + Category
        </button>
      </div>

      {open && (
        <div className="space-y-3 border-t border-(--color-line) px-4 pt-3 pb-4">
          <p className="text-xs text-(--color-muted)">Category → Subcategory → Variety{example}</p>
          {adding && (
            <InlineAdd
              placeholder={`Category name under ${stage.name} (e.g. ${stage.code === "bought_out" ? "Packaging, Ceramics, Goodies" : "Woodbased, Magnets"})`}
              saveLabel="Save category"
              busy={ctx.isPending}
              className="bg-emerald-50/50"
              onSave={handleAdd}
              onCancel={() => setAdding(false)}
            />
          )}
          <div className="space-y-3">
            {stage.categories.map((cat) => (
              <CategoryBlock key={cat.id} cat={cat} isMachine={isMachine} stageCode={stage.code} ctx={ctx} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
