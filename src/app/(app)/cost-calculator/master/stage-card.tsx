"use client";

import { useState } from "react";
import type { CostStageWithHierarchy } from "@/lib/types";
import { saveCostCategory } from "./actions";
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

  return (
    <div className="card p-5 space-y-4">
      <div
        className={`flex flex-wrap items-center justify-between gap-3 ${open ? "border-b border-(--color-line) pb-3" : ""}`}
      >
        <button
          type="button"
          onClick={() => ctx.toggle(stage.id)}
          aria-expanded={open}
          className="text-left"
        >
          <div className="flex items-center gap-2">
            <Chevron open={open} />
            <span className="flex h-6 w-6 items-center justify-center rounded-full bg-(--color-brand) text-xs font-bold text-white">
              {stage.sort_order}
            </span>
            <h3 className="text-lg font-bold text-(--color-ink)">{stage.name} Master</h3>
            <span className="badge text-[11px]">
              {stage.categories.length}{" "}
              {stage.categories.length === 1 ? "category" : "categories"}
            </span>
          </div>
          <p className="text-xs text-(--color-muted) mt-0.5">
            {stage.code === "bought_out"
              ? "Hierarchy: Category (e.g. Ceramics) → Subcategory (e.g. Jars) → Varieties (e.g. 200ml Amber)"
              : "Hierarchy: Category → Subcategory (e.g. Birch) → Varieties (e.g. 3mm, 8mm, 12mm)"}
          </p>
        </button>

        <button
          type="button"
          onClick={() => {
            setAdding(true);
            ctx.expand(stage.id);
          }}
          className="btn-secondary text-xs py-1.5 px-3"
        >
          + Add Category to {stage.name}
        </button>
      </div>

      {open && (
        <>
          {adding && (
            <InlineAdd
              placeholder={`Category name under ${stage.name} (e.g. ${stage.code === "bought_out" ? "Packaging, Ceramics, Goodies" : "Woodbased, Magnets"})`}
              saveLabel="Save Category"
              busy={ctx.isPending}
              className="bg-emerald-50/50"
              onSave={handleAdd}
              onCancel={() => setAdding(false)}
            />
          )}
          <div className="space-y-4">
            {stage.categories.map((cat) => (
              <CategoryBlock key={cat.id} cat={cat} isMachine={isMachine} stageCode={stage.code} ctx={ctx} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
