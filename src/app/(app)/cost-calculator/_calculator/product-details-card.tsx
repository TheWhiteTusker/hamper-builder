"use client";

import Image from "next/image";
import Link from "@/components/link";
import { formatMoney } from "@/lib/pricing";
import type { ProductColor } from "@/lib/product-code";
import type { Category } from "@/lib/types";
import { costingHref } from "../href";
import type { Sibling } from "./apply-price-dialog";
import { CodeField } from "./code-field";
import { ColorPhotoRows } from "./color-photo-rows";
import { ORIGINS } from "./lines";
import type { ProductDetails } from "./use-product-details";
import type { ProductPhotos } from "./use-product-photos";
import { thumbProps } from "@/lib/product-image-props";

export function ProductDetailsCard({
  details,
  photos,
  siblings,
  categories,
  productColors,
  imageUrl,
}: {
  details: ProductDetails;
  photos: ProductPhotos;
  /** This product in its other colours: separate products, linked for switching. */
  siblings: Sibling[];
  categories: Category[];
  productColors: ProductColor[];
  imageUrl?: string | null;
}) {
  const { source } = details;
  return (
    <div className="card p-5">
      <div className="flex flex-wrap items-center justify-between gap-4 border-b border-[var(--color-border)] pb-4">
        <div className="flex items-center gap-3">
          {imageUrl && (
            <div className="relative h-12 w-12 shrink-0 overflow-hidden rounded-lg border border-slate-200 bg-slate-100 shadow-xs">
              <Image {...thumbProps(imageUrl)} alt={details.name || "Product photo"} fill sizes="48px" className="object-cover" unoptimized />
            </div>
          )}
          <div>
            <h2 className="text-lg font-bold text-[var(--color-ink)]">Product & Costing Specification</h2>
            <p className="text-sm text-[var(--color-muted)]">
              {details.editing
                ? "Each colour is its own product with its own costing. Changes here save to this colour only."
                : "Code format CATEGORY/0001/COLOUR (e.g. LC/0001/WL). Tick several colours to create one product per colour."}
            </p>
          </div>
        </div>

        {siblings.length > 0 && (
          <div className="flex flex-wrap items-center gap-1.5 text-xs">
            <span className="font-semibold text-[var(--color-muted)]">Other colours:</span>
            {siblings.map((s) => (
              <Link
                key={s.id}
                href={costingHref(s.code)}
                className="rounded-full bg-[var(--color-sheet)] px-2.5 py-1 font-medium hover:bg-slate-200"
                title={`Edit ${s.color} (a separate product)`}
              >
                {s.color} <span className="font-mono">{formatMoney(s.sp)}</span>
              </Link>
            ))}
          </div>
        )}
      </div>

      <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <CodeField details={details} productColors={productColors} />

        <div>
          <label className="label" htmlFor="calc-category">
            Product Master Category
          </label>
          <select
            id="calc-category"
            value={details.categoryId}
            onChange={(e) => details.changeCategory(e.target.value)}
            disabled={details.editing}
            title={details.editing ? "Part of the product code, so fixed once created" : undefined}
            className="select mt-1"
          >
            <option value="">No category</option>
            {categories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.code ? `[${c.code}] ` : ""}
                {c.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="label" htmlFor="calc-name">
            Product Name *
          </label>
          <input
            id="calc-name"
            value={details.name}
            onChange={(e) => details.setName(e.target.value)}
            placeholder="e.g. Lotus Glow (Pair)"
            required
            className="input mt-1"
          />
        </div>

        <div>
          <label className="label" htmlFor="calc-source">
            Product Origin
          </label>
          <select
            id="calc-source"
            value={source}
            onChange={(e) => details.changeOrigin(e.target.value)}
            className="select mt-1"
          >
            {ORIGINS.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
          <p className="mt-1 text-[11px] text-[var(--color-muted)]">
            {source === "In-house"
              ? "In-house manufacturing stages and bought-out items apply."
              : source === "Outsource"
                ? "Manufacturing stages start collapsed — type each item under Bought Out Items; new ones are saved to the Rates page."
                : "In-house stages plus bought-out items with their own markup."}
          </p>
        </div>
      </div>

      <div className="mt-4 border-t border-[var(--color-border)] pt-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-xs font-semibold text-[var(--color-ink)]">
            {details.editing ? "Colour & photos" : "Colours to create & photos"}
          </span>
          <label className="flex items-center gap-2 text-sm font-medium text-[var(--color-ink)] cursor-pointer">
            <input
              type="checkbox"
              checked={details.isActive}
              onChange={(e) => details.setIsActive(e.target.checked)}
              className="h-4 w-4 rounded text-[var(--color-brand)] focus:ring-[var(--color-brand)]"
            />
            <span>Active in Product Master & Hamper Builder</span>
          </label>
        </div>
        {!details.selectedProductId && (
          <p className="mt-1 text-xs text-[var(--color-muted)]">
            Photos you add now upload automatically when you save the product.
          </p>
        )}
        <ColorPhotoRows details={details} photos={photos} productColors={productColors} />
      </div>
    </div>
  );
}
