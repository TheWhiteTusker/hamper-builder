"use client";

import { useState } from "react";
import { Table, TableBody } from "@/components/ui/table";
import type { Category } from "@/lib/types";
import { AddCategoryForm, CategoryRow } from "./category-forms";

/** The categories table. Saves patch this list instead of re-rendering the page. */
export function CategoryList({ categories: initial }: { categories: Category[] }) {
  const [categories, setCategories] = useState(initial);
  const [prevInitial, setPrevInitial] = useState(initial);
  if (initial !== prevInitial) {
    setPrevInitial(initial);
    setCategories(initial);
  }
  const nextSortOrder = categories.reduce((max, c) => Math.max(max, c.sort_order), -1) + 1;

  return (
    <>
      <Table>
        <TableBody>
          {categories.map((category) => (
            <CategoryRow
              key={category.id}
              category={category}
              onSaved={(saved) => setCategories((list) => list.map((c) => (c.id === saved.id ? saved : c)))}
              onDeleted={(id) => setCategories((list) => list.filter((c) => c.id !== id))}
            />
          ))}
        </TableBody>
      </Table>

      {categories.length === 0 && (
        <p className="px-4 py-6 text-sm text-(--color-muted)">
          No categories yet. Import the Settings tab, or add them here.
        </p>
      )}

      <div className="border-t border-line">
        <AddCategoryForm
          nextSortOrder={nextSortOrder}
          onAdded={(added) => setCategories((list) => [...list, added])}
        />
      </div>
    </>
  );
}
