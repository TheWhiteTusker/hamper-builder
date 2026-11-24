"use client";

import { TableBody } from "@/components/ui/table";
import { useLiveRows } from "@/components/use-live";
import type { Category, ProductWithCategory } from "@/lib/types";
import { ProductRow } from "./product-row";

/** The table's rows, patched the moment someone else edits or bins a product. */
export function ProductRows({
  rows: serverRows,
  categories,
  admin,
}: {
  rows: ProductWithCategory[];
  categories: Category[];
  admin: boolean;
}) {
  const rows = useLiveRows(serverRows, "products", (cur, row) => {
    const cat = categories.find((c) => c.id === row.category_id);
    return {
      ...cur,
      ...row,
      categories: cat ? { name: cat.name, counts_as_item: cat.counts_as_item } : null,
    } as ProductWithCategory;
  });

  return (
    <TableBody>
      {rows.map((p, i) => (
        <ProductRow key={p.id} p={p} n={i + 1} admin={admin} />
      ))}
    </TableBody>
  );
}
