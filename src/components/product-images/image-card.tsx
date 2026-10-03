"use client";

import Image from "next/image";
import { FALLBACK_COLOR_HEX, type ProductColor } from "@/lib/product-code";
import type { ProductImage } from "@/lib/types";
import { thumbProps } from "@/lib/product-images";

/** One photo: cover badge, delete, its colour tag and "Make Primary". */
export function ImageCard({
  img,
  colors,
  color,
  productName,
  onDelete,
  onColor,
  onPrimary,
}: {
  img: ProductImage;
  colors: ProductColor[];
  /** The finish this photo is tagged with, if any. */
  color?: ProductColor;
  productName?: string;
  onDelete: () => void;
  onColor: (color: string) => void;
  onPrimary: () => void;
}) {
  return (
    <div
      className={`group relative overflow-hidden rounded-xl border bg-white shadow-xs transition-all ${
        img.is_primary
          ? "border-[var(--color-brand)] ring-2 ring-[var(--color-brand)]/20"
          : "border-slate-200 hover:border-slate-300"
      }`}
    >
      <div className="relative aspect-square w-full bg-slate-100 overflow-hidden">
        <Image
          {...thumbProps(img.url)}
          alt={img.caption || productName || "Product Photo"}
          fill
          sizes="(max-width: 640px) 50vw, (max-width: 768px) 33vw, 25vw"
          className="object-cover transition-transform duration-200 group-hover:scale-105"
          unoptimized
        />

        {img.is_primary && (
          <span className="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-emerald-700/90 px-2 py-0.5 text-[10px] font-bold text-white shadow-xs backdrop-blur-xs">
            ★ Primary
          </span>
        )}

        <button
          type="button"
          onClick={onDelete}
          className="absolute top-2 right-2 flex h-6 w-6 items-center justify-center rounded-full bg-white/90 text-slate-500 shadow-sm hover:bg-red-50 hover:text-red-600 transition-colors opacity-0 group-hover:opacity-100"
          title="Delete photo"
        >
          &times;
        </button>
      </div>

      <div className="p-2 space-y-1.5">
        <div className="flex items-center justify-between gap-1">
          <div className="flex items-center gap-1.5 min-w-0">
            <span
              className="h-2 w-2 rounded-full border border-black/20 shrink-0"
              style={{ backgroundColor: color?.hex || FALLBACK_COLOR_HEX }}
            />
            <select
              value={img.color || "General"}
              onChange={(e) => onColor(e.target.value)}
              className="text-[11px] font-medium text-slate-700 bg-transparent border-0 p-0 focus:ring-0 cursor-pointer truncate"
            >
              {colors.map((c) => (
                <option key={c.name} value={c.name}>
                  {c.name} ({c.code})
                </option>
              ))}
              <option value="General">General / All</option>
            </select>
          </div>

          {!img.is_primary && (
            <button
              type="button"
              onClick={onPrimary}
              className="text-[10px] font-medium text-slate-400 hover:text-[var(--color-brand)] transition-colors"
              title="Set as primary cover photo"
            >
              Make Primary
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
