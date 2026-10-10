"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { useActionState } from "react";
import type { Category } from "@/lib/types";
import { saveCategory, deleteCategory } from "../category-actions";
import { Status } from "./status";

export function CategoryRow({ category }: { category: Category }) {
  const [state, action, pending] = useActionState(saveCategory, {});
  const [delState, delAction, deleting] = useActionState(deleteCategory, {});

  return (
    <TableRow>
      <TableCell colSpan={4} className="p-0">
        <div className="flex flex-wrap items-center gap-2 px-2.5 py-1.5">
          <form action={action} className="flex flex-1 flex-wrap items-center gap-2">
            <input type="hidden" name="id" value={category.id} />
            <input type="hidden" name="sort_order" value={category.sort_order} />

            <input
              name="code"
              defaultValue={category.code ?? ""}
              placeholder="Code"
              aria-label="Category code"
              className="input max-w-[70px] uppercase font-mono font-bold text-center text-xs"
              title="2-letter category code used in product codes (e.g. LC, ED)"
            />

            <input
              name="name"
              defaultValue={category.name}
              aria-label="Category name"
              className="input max-w-[220px]"
            />

            <label className="flex items-center gap-1.5 text-sm whitespace-nowrap">
              <input
                type="checkbox"
                name="counts_as_item"
                defaultChecked={category.counts_as_item}
              />
              Counts as an item
            </label>

            <button type="submit" className="btn-secondary" disabled={pending}>
              {pending ? "Saving…" : "Save"}
            </button>
            <Status state={state} />
          </form>

          <form action={delAction} className="flex items-center gap-2">
            <input type="hidden" name="id" value={category.id} />
            <button type="submit" className="btn-danger" disabled={deleting}>
              Delete
            </button>
            {delState.error && (
              <span role="alert" className="text-sm text-red-700">
                {delState.error}
              </span>
            )}
          </form>
        </div>
      </TableCell>
    </TableRow>
  );
}

export function AddCategoryForm({ nextSortOrder }: { nextSortOrder: number }) {
  const [state, action, pending] = useActionState(saveCategory, {});

  return (
    <form action={action} className="flex flex-wrap items-center gap-2 p-3">
      <input type="hidden" name="sort_order" value={nextSortOrder} />

      <input
        name="code"
        placeholder="Code (e.g. LC)"
        aria-label="Category code"
        className="input max-w-[90px] uppercase font-mono font-bold text-center text-xs"
        title="2-letter category code used in product codes (e.g. LC, ED)"
      />

      <input
        name="name"
        placeholder="New category name"
        aria-label="New category name"
        className="input max-w-[220px]"
        required
      />

      <label className="flex items-center gap-1.5 text-sm whitespace-nowrap">
        <input type="checkbox" name="counts_as_item" defaultChecked />
        Counts as an item
      </label>

      <button type="submit" className="btn-primary" disabled={pending}>
        {pending ? "Adding…" : "Add category"}
      </button>
      <Status state={state} />
    </form>
  );
}
