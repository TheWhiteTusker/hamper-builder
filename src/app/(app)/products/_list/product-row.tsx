"use client";

import { memo } from "react";
import { TableCell, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { formatMoney } from "@/lib/pricing";
import { ImagePreview } from "@/components/image-preview";
import type { ProductWithCategory } from "@/lib/types";
import { costingHref } from "../../cost-calculator/href";
import { productHref } from "../href";
import { DeleteProductButton } from "./delete-product-button";
import { ReviewMark, SelectBox } from "@/components/list-select/row-select";

/** One product in the Product Master table. Memo: a live change re-renders only its own row. */
export const ProductRow = memo(function ProductRow({ p, admin }: { p: ProductWithCategory; admin: boolean }) {
  return (
    <TableRow
      // Ticked: the brand tint. Marked for review: red. Never both (selection.ts);
      // the ! only covers the instant between the two writes.
      className={`group has-checked:bg-(--color-brand)/15 has-aria-pressed:bg-red-100! ${p.is_active ? "" : "opacity-55"}`}
    >
      <TableCell className="relative w-12 py-1.5 px-2 text-center">
        {/* In the gutter left of the table (.row-gutter). Both show only on row hover or
            keyboard focus (always on touch screens, which can't hover); the row's colour
            shows what's ticked or marked. */}
        <div className="absolute top-1/2 right-full mr-1.5 flex -translate-y-1/2 items-center gap-1">
          <SelectBox
            kind="products"
            id={p.id}
            label={p.code}
            className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 pointer-coarse:opacity-100"
          />
          <ReviewMark kind="products" id={p.id} label={p.code} />
        </div>
        {p.image_url ? (
          <ImagePreview
            src={p.image_url}
            alt={p.name}
            sizes="36px"
            className="mx-auto h-9 w-9 rounded-md border border-slate-200 bg-slate-100 shadow-2xs hover:ring-2 hover:ring-(--color-brand) transition-all"
          />
        ) : (
          <Link
            href={costingHref(p.code)}
            title={`Add photos for ${p.name}`}
            className="mx-auto flex h-9 w-9 items-center justify-center rounded-md border border-dashed border-slate-300 bg-slate-50 text-[10px] font-semibold text-slate-400 hover:border-(--color-brand) hover:text-(--color-brand) transition-colors"
          >
            + Pic
          </Link>
        )}
      </TableCell>
      <TableCell className="font-mono whitespace-nowrap">
        <Link
          href={productHref(p.code)}
          className="font-semibold text-(--color-ink) hover:text-(--color-brand) hover:underline"
        >
          {p.code}
        </Link>
      </TableCell>
      <TableCell>
        <Link
          href={productHref(p.code)}
          className="font-medium hover:underline text-(--color-ink)"
        >
          {p.name}
        </Link>
        {!p.is_active && <span className="badge ml-2">Inactive</span>}
      </TableCell>
      <TableCell className="text-(--color-muted)">{p.categories?.name ?? "—"}</TableCell>
      <TableCell className="text-(--color-muted)">{p.source ?? "—"}</TableCell>
      <TableCell className="num">{formatMoney(p.cost_price)}</TableCell>
      <TableCell className="num">{p.markup_pct != null ? `${p.markup_pct}%` : "—"}</TableCell>
      <TableCell className="num">{formatMoney(p.default_sp)}</TableCell>
      <TableCell className="num">
        <div className="flex items-center justify-end gap-2.5">
          {/* Editing a product happens in the cost calculator. */}
          {admin ? (
            <>
              <Link
                href={costingHref(p.code)}
                className="font-medium text-(--color-ink) hover:text-(--color-brand) hover:underline"
              >
                Edit
              </Link>
              <DeleteProductButton id={p.id} label={`${p.code} (${p.name})`} />
            </>
          ) : (
            <Link href={costingHref(p.code)} className="text-(--color-brand) hover:underline font-medium">
              Costing
            </Link>
          )}
        </div>
      </TableCell>
    </TableRow>
  );
});
