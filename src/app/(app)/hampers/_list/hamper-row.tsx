"use client";

import { memo } from "react";
import { TableCell, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { formatMoney, formatPct } from "@/lib/pricing";
import type { HamperSummary } from "@/lib/types";
import { ReviewMark, SelectBox } from "@/components/list-select/row-select";

/** One hamper in the list: picture, figures, and red where it loses money. Memo: a live change re-renders only its own row. */
export const HamperRow = memo(function HamperRow({ h, n }: { h: HamperSummary; n: number }) {
  return (
    <TableRow
      // Ticked: the brand tint. Marked for review: red. Never both (selection.ts);
      // the ! only covers the instant between the two writes. As the Product Master.
      className="group has-checked:bg-(--color-brand)/15 has-aria-pressed:bg-red-100!"
    >
      <TableCell className="num relative w-12 text-(--color-muted)">
        {n}
        {/* In the gutter left of the table (.row-gutter), shown on row hover or keyboard
            focus (always on touch screens); the row's colour shows what's ticked or marked. */}
        <div className="absolute top-1/2 right-full mr-1.5 flex -translate-y-1/2 items-center gap-1">
          <SelectBox
            kind="hampers"
            id={h.id}
            label={h.code}
            className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 pointer-coarse:opacity-100"
          />
          <ReviewMark kind="hampers" id={h.id} label={h.code} />
        </div>
      </TableCell>
      <TableCell className="w-14">
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="block h-10 w-10">
          {h.image_url ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={h.image_url}
              alt=""
              loading="lazy"
              className="h-10 w-10 rounded-md border border-(--color-line) object-cover"
            />
          ) : (
            <span className="block h-10 w-10 rounded-md border border-dashed border-(--color-line) bg-paper" />
          )}
        </Link>
      </TableCell>
      <TableCell className="font-mono whitespace-nowrap">
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.code}
        </Link>
      </TableCell>
      <TableCell>
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.name}
        </Link>
      </TableCell>
      <TableCell className="text-(--color-muted)">{h.collection ?? "—"}</TableCell>
      <TableCell>
        <span className="badge">{h.status}</span>
      </TableCell>
      <TableCell className="num">{h.number_of_items}</TableCell>
      <TableCell className="num">{formatMoney(h.total_cp)}</TableCell>
      <TableCell className="num">
        {h.final_catalogue_sp == null ? (
          <span className="text-(--color-muted)">not priced</span>
        ) : (
          formatMoney(h.final_catalogue_sp)
        )}
      </TableCell>
      <TableCell
        className={`num ${h.gross_profit != null && h.gross_profit < 0 ? "text-red-700" : ""}`}
      >
        {h.gross_profit == null ? "—" : formatMoney(h.gross_profit)}
      </TableCell>
      <TableCell
        className={`num ${h.final_margin != null && h.final_margin < 0 ? "text-red-700" : ""}`}
      >
        {h.final_margin == null ? "—" : formatPct(h.final_margin)}
      </TableCell>
    </TableRow>
  );
});
