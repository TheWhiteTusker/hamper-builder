"use client";

import { useState } from "react";
import type { CostCategoryWithSubcategories } from "@/lib/types";
import { deleteCostCategory, renameCostCategory, saveCostSubcategory } from "./actions";
import { Chevron, InlineAdd, type MasterCtx } from "./master-ui";
import { SubcategoryBlock } from "./subcategory-block";

export function CategoryBlock({
  cat,
  isMachine,
  stageCode,
  ctx,
}: {
  cat: CostCategoryWithSubcategories;
  isMachine: boolean;
  stageCode?: string;
  ctx: MasterCtx;
}) {
  const [adding, setAdding] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(cat.name);
  const open = ctx.isOpen(cat.id);
  const isBoughtOut = stageCode === "bought_out";

  function handleAdd(subName: string) {
    const fd = new FormData();
    fd.set("category_id", cat.id);
    fd.set("name", subName);
    ctx.run(() => saveCostSubcategory(fd), "Subcategory added successfully!", () => setAdding(false));
  }

  function handleRename(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (!trimmed || trimmed === cat.name) {
      setIsEditing(false);
      setName(cat.name);
      return;
    }
    ctx.run(
      () => renameCostCategory(cat.id, trimmed),
      `Category renamed to "${trimmed}".`,
      () => setIsEditing(false),
    );
  }

  function handleDelete() {
    if (
      !window.confirm(
        `Are you sure you want to delete category "${cat.name}" and all its subcategories and varieties?`,
      )
    )
      return;
    ctx.run(() => deleteCostCategory(cat.id), `Category "${cat.name}" deleted.`);
  }

  return (
    <div className="rounded-xl border border-(--color-line) bg-slate-50/40 p-4 space-y-3 min-w-0 max-w-full">
      <div
        className={`flex flex-wrap items-center justify-between gap-2 ${open ? "border-b border-(--color-line) pb-2" : ""}`}
      >
        {isEditing ? (
          <form onSubmit={handleRename} className="flex min-w-0 flex-1 flex-wrap items-center gap-2 py-0.5">
            <span className="text-xs font-semibold text-(--color-muted) uppercase tracking-wide">
              Category:
            </span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              placeholder="Category name"
              className="input text-xs py-1 font-bold uppercase tracking-wide w-full sm:w-auto sm:max-w-xs"
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
                setName(cat.name);
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
              onClick={() => ctx.toggle(cat.id)}
              aria-expanded={open}
              className="flex min-w-0 items-center gap-2 text-left"
            >
              <Chevron open={open} />
              <span className="font-bold text-sm text-(--color-ink) uppercase tracking-wide truncate">
                {cat.name}
              </span>
              <span className="badge shrink-0 text-[11px]">
                {cat.subcategories.length}{" "}
                {cat.subcategories.length === 1 ? "subcategory" : "subcategories"}
              </span>
            </button>

            <div className="flex shrink-0 flex-wrap items-center gap-2">
              <button
                type="button"
                onClick={() => {
                  setIsEditing(true);
                  setName(cat.name);
                }}
                className="btn-secondary text-xs py-1 px-2.5"
                title="Rename Category"
              >
                Rename
              </button>
              <button
                type="button"
                onClick={() => {
                  setAdding(true);
                  ctx.expand(cat.id);
                }}
                className="btn-secondary text-xs py-1 px-2.5"
              >
                + Add Subcategory
              </button>
              <button
                type="button"
                onClick={handleDelete}
                className="text-xs text-red-600 hover:text-red-800 p-1"
                title="Delete Category"
              >
                Delete Category
              </button>
            </div>
          </>
        )}
      </div>

      {open && (
        <>
          {adding && (
            <InlineAdd
              placeholder={`Subcategory under ${cat.name} (e.g. ${isBoughtOut ? "Glass Bottles, Scented Candles" : "Birch, Rubberwood"})`}
              saveLabel="Save Subcategory"
              busy={ctx.isPending}
              className="bg-white"
              onSave={handleAdd}
              onCancel={() => setAdding(false)}
            />
          )}

          {cat.subcategories.length === 0 ? (
            <p className="text-xs text-(--color-muted) italic py-2">
              No subcategories in {cat.name} yet. Click &ldquo;+ Add Subcategory&rdquo; above.
            </p>
          ) : (
            <div className="space-y-3">
              {cat.subcategories.map((sub) => (
                <SubcategoryBlock
                  key={sub.id}
                  sub={sub}
                  categoryId={cat.id}
                  isMachine={isMachine}
                  stageCode={stageCode}
                  ctx={ctx}
                />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
