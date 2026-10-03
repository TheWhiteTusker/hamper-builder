"use client";

import Image from "next/image";
import type { ProductColor } from "@/lib/product-code";
import type { Category, Product } from "@/lib/types";
import { CodeField } from "./code-field";
import { ColorPhotoRows } from "./color-photo-rows";
import { ORIGINS } from "./lines";
import type { ProductDetails } from "./use-product-details";
import type { ProductPhotos } from "./use-product-photos";
import { thumbProps } from "@/lib/product-images";

export function ProductDetailsCard({
  details,
  photos,
  products,
  categories,
  productColors,
  imageUrl,
}: {
  details: ProductDetails;
  photos: ProductPhotos;
  products: Product[];
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
              Standard format: [CATEGORY]/[0001]/[COLOR] (e.g. LC/0001/WL). Each product comes in 3 colors only (Walnut, Natural, Black).
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <label className="text-xs font-semibold uppercase tracking-wider text-[var(--color-muted)]">
            Load Existing:
          </label>
          <select
            value={details.selectedProductId}
            onChange={(e) => details.selectProduct(e.target.value)}
            className="select text-sm py-1.5 min-w-[220px]"
          >
            <option value="">-- Create New Product --</option>
            {products.map((p) => (
              <option key={p.id} value={p.id}>
                {p.code} - {p.name}
              </option>
            ))}
          </select>
        </div>
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
                ? "Manufacturing stages start collapsed — cost items under Bought Out Items."
                : "In-house stages plus bought-out items with their own markup."}
          </p>
        </div>
      </div>

      <div className="mt-4 border-t border-[var(--color-border)] pt-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-xs font-semibold text-[var(--color-ink)]">Color Finishes &amp; Photos</span>
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
