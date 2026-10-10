"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { useTransition } from "react";
import Image from "next/image";
import type { BinItem } from "./bin-types";
import { restoreBinItem } from "./bin-restore";
import { permanentlyDeleteBinItem } from "./bin-delete";
import { thumbProps } from "@/lib/product-images";

export function BinItemRow({
  item,
  canManage,
  isAdmin,
  onNotify,
}: {
  item: BinItem;
  canManage: boolean;
  isAdmin: boolean;
  onNotify: (msg: { text: string; error?: boolean }) => void;
}) {
  const [isPending, startTransition] = useTransition();

  const canAction =
    item.type === "product" ? isAdmin : canManage;

  function handleRestore() {
    startTransition(async () => {
      const res = await restoreBinItem(item.id, item.type);
      if (res.error) {
        onNotify({ text: res.error, error: true });
      } else {
        onNotify({ text: res.message || "Item restored successfully." });
      }
    });
  }

  function handleDeletePermanently() {
    if (
      !confirm(
        `Permanently delete "${item.title}"? This will delete all its data and storage files and CANNOT be undone.`,
      )
    ) {
      return;
    }

    startTransition(async () => {
      const res = await permanentlyDeleteBinItem(item.id, item.type);
      if (res.error) {
        onNotify({ text: res.error, error: true });
      } else {
        onNotify({ text: res.message || "Item permanently deleted." });
      }
    });
  }

  const badgeColor =
    item.type === "hamper"
      ? "bg-[#54655b]/10 text-[#54655b] border-[#54655b]/30"
      : item.type === "product"
        ? "bg-amber-100/80 text-amber-900 border-amber-300"
        : "bg-blue-100/80 text-blue-900 border-blue-300";

  const typeLabel =
    item.type === "hamper"
      ? "Hamper"
      : item.type === "product"
        ? "Product"
        : "Photo";

  const expiryBadge =
    item.daysRemaining === 0 ? (
      <span className="inline-flex items-center gap-1 rounded-full border border-red-300 bg-red-50 px-2.5 py-0.5 text-xs font-semibold text-red-700">
        Expired (0d)
      </span>
    ) : item.daysRemaining <= 3 ? (
      <span className="inline-flex items-center gap-1 rounded-full border border-rose-300 bg-rose-50 px-2.5 py-0.5 text-xs font-semibold text-rose-700">
        {item.daysRemaining} {item.daysRemaining === 1 ? "day" : "days"} left
      </span>
    ) : item.daysRemaining <= 10 ? (
      <span className="inline-flex items-center gap-1 rounded-full border border-amber-300 bg-amber-50 px-2.5 py-0.5 text-xs font-medium text-amber-800">
        {item.daysRemaining} days left
      </span>
    ) : (
      <span className="inline-flex items-center gap-1 rounded-full border border-line bg-paper px-2.5 py-0.5 text-xs text-(--color-muted)">
        {item.daysRemaining} days left
      </span>
    );

  const formattedDate = new Date(item.deletedAt).toLocaleDateString("en-IN", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });

  return (
    <TableRow className={`transition-opacity ${isPending ? "opacity-40" : ""}`}>
      {/* Thumbnail */}
      <TableCell className="w-14 py-2.5">
        <div className="relative h-12 w-12 shrink-0 overflow-hidden rounded-md border border-line bg-paper">
          {item.imageUrl ? (
            <Image
              {...thumbProps(item.imageUrl)}
              alt=""
              fill
              sizes="48px"
              className="object-cover"
              unoptimized
            />
          ) : (
            <div className="flex h-full w-full items-center justify-center text-xs text-(--color-muted)">
              No pic
            </div>
          )}
        </div>
      </TableCell>

      {/* Type */}
      <TableCell className="w-24 whitespace-nowrap py-2.5">
        <span
          className={`inline-block rounded-full border px-2.5 py-0.5 text-[11px] font-medium tracking-wide uppercase ${badgeColor}`}
        >
          {typeLabel}
        </span>
      </TableCell>

      {/* Code */}
      <TableCell className="w-32 whitespace-nowrap font-mono text-xs font-medium text-(--color-ink) py-2.5">
        {item.code || "—"}
      </TableCell>

      {/* Title & Details */}
      <TableCell className="py-2.5">
        <div className="font-medium text-(--color-ink)">{item.title}</div>
        {item.subtitle && (
          <div className="text-xs text-(--color-muted)">{item.subtitle}</div>
        )}
      </TableCell>

      {/* Deleted Date */}
      <TableCell className="whitespace-nowrap text-xs text-(--color-muted) py-2.5">
        {formattedDate}
      </TableCell>

      {/* Retention / Expiry */}
      <TableCell className="whitespace-nowrap py-2.5">{expiryBadge}</TableCell>

      {/* Actions */}
      <TableCell className="whitespace-nowrap text-right py-2.5">
        {canAction ? (
          <div className="inline-flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={handleRestore}
              disabled={isPending}
              className="btn-secondary text-xs px-3 py-1 font-medium hover:border-(--color-brand) hover:text-(--color-brand)"
              title="Restore to active use"
            >
              Restore
            </button>
            <button
              type="button"
              onClick={handleDeletePermanently}
              disabled={isPending}
              className="btn-danger text-xs px-3 py-1 font-medium"
              title="Permanently remove"
            >
              Delete
            </button>
          </div>
        ) : (
          <span className="text-xs text-(--color-muted)">Read-only</span>
        )}
      </TableCell>
    </TableRow>
  );
}
