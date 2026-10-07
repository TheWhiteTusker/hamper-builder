"use client";

import { useActionState, useState } from "react";
import Link from "@/components/link";
import { saveProduct, deleteProduct } from "./actions";
import { STANDARD_PRODUCT_COLORS, resolveColors, type ProductColor } from "@/lib/product-code";
import { ProductImagesManager } from "@/components/product-images-manager";
import type { Category, Product, ProductImage } from "@/lib/types";
import { costingHref } from "../cost-calculator/href";
import { CodeField } from "./_form/code-field";
import { ColorPicker, SourceField } from "./_form/color-picker";
import { PriceFields } from "./_form/price-fields";
import { useAutoCode } from "./_form/use-auto-code";

export function ProductForm({
  product,
  initialImages = [],
  categories,
  sources,
  allColors = resolveColors(STANDARD_PRODUCT_COLORS.map((c) => c.name)),
}: {
  product?: Product;
  initialImages?: ProductImage[];
  categories: Category[];
  sources: string[];
  allColors?: ProductColor[];
}) {
  const [state, action, pending] = useActionState(saveProduct, {});
  const [deleteState, deleteAction, deleting] = useActionState(deleteProduct, {});

  const [categoryId, setCategoryId] = useState(product?.category_id ?? "");
  const [source, setSource] = useState(product?.source ?? "");
  const [selectedColors, setSelectedColors] = useState<string[]>(
    product?.colors && product.colors.length > 0 ? product.colors : ["Walnut"],
  );
  const auto = useAutoCode(product?.code ?? "", categories, selectedColors);
  const code = auto.code;

  function handleCategoryChange(newCatId: string) {
    setCategoryId(newCatId);
    auto.categoryChanged(newCatId);
  }

  function handleColorSelect(colName: string) {
    const next = selectedColors.includes(colName)
      ? selectedColors.filter((c) => c !== colName)
      : [...selectedColors, colName];
    setSelectedColors(next);
    auto.colorsChanged(next);
  }

  return (
    <>
      {product && (
        <div className="mb-4 max-w-2xl flex flex-wrap items-center justify-between gap-3 rounded-xl bg-emerald-50/70 p-4 border border-[var(--color-brand)]/30">
          <div>
            <span className="text-xs font-bold uppercase tracking-wider text-[var(--color-brand-dark)]">
              Multi-Stage Costing
            </span>
            <p className="text-xs text-[var(--color-muted)] mt-0.5">
              Break down this product into Material, Hardware, Finishing & Machine per-minute costs.
            </p>
          </div>
          <Link href={costingHref(product.code)} className="btn-primary text-xs py-1.5 px-3">
            Open in Cost Calculator &rarr;
          </Link>
        </div>
      )}

      <form action={action} className="card max-w-2xl p-5">
        {product && <input type="hidden" name="id" value={product.id} />}
        {selectedColors.map((col) => (
          <input key={col} type="hidden" name="colors" value={col} />
        ))}

        <div className="grid gap-4 sm:grid-cols-2">
          <CodeField code={code} onChange={auto.setCode} fetching={auto.fetching} />

          <div>
            <label className="label" htmlFor="category_id">
              Category
            </label>
            <select
              id="category_id"
              name="category_id"
              value={categoryId}
              onChange={(e) => handleCategoryChange(e.target.value)}
              className="select mt-1"
            >
              <option value="">No category</option>
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.code ? `[${c.code}] ` : ""}
                  {c.name}
                  {c.counts_as_item ? "" : " (not counted as an item)"}
                </option>
              ))}
            </select>
          </div>

          <div className="sm:col-span-2">
            <label className="label" htmlFor="name">
              Product name
            </label>
            <input id="name" name="name" required defaultValue={product?.name} className="input mt-1" />
          </div>

          <SourceField value={source} onChange={setSource} sources={sources} />
          <ColorPicker allColors={allColors} selected={selectedColors} code={code} isNew={!product} onToggle={handleColorSelect} />
          <PriceFields product={product} />

          <label className="sm:col-span-2 flex items-center gap-2 py-1 text-sm cursor-pointer">
            <input
              type="checkbox"
              name="is_active"
              defaultChecked={product ? product.is_active : true}
              className="h-4 w-4 rounded text-[var(--color-brand)] focus:ring-[var(--color-brand)]"
            />
            <span className="font-medium">Active — available when building hampers</span>
          </label>
        </div>

        {state.error && (
          <p role="alert" className="mt-4 text-sm text-red-700 font-medium">
            {state.error}
          </p>
        )}

        <div className="mt-5 flex items-center gap-2">
          <button type="submit" className="btn-primary" disabled={pending}>
            {pending
              ? "Saving…"
              : product
                ? "Save changes"
                : selectedColors.length > 1
                  ? `Create ${selectedColors.length} products`
                  : "Create product"}
          </button>
          <Link href="/products" className="btn-secondary">
            Cancel
          </Link>
        </div>
      </form>

      <div className="card max-w-2xl p-5 mt-4">
        <ProductImagesManager
          productId={product?.id}
          initialImages={initialImages}
          productName={product?.name}
          currentColor={selectedColors[0]}
          colors={selectedColors.length ? allColors.filter((c) => selectedColors.includes(c.name)) : allColors}
        />
      </div>

      {product && (
        <form
          action={deleteAction}
          className="mt-4 max-w-2xl"
          onSubmit={(e) => {
            if (!confirm(`Move ${product.name} to the Bin? It will stay in the Bin for 30 days and can be restored anytime.`)) {
              e.preventDefault();
            }
          }}
        >
          <input type="hidden" name="id" value={product.id} />
          {deleteState.error && (
            <p role="alert" className="mb-2 text-sm text-red-700">
              {deleteState.error}
            </p>
          )}
          <button type="submit" className="btn-danger" disabled={deleting}>
            {deleting ? "Moving to Bin…" : "Move to Bin"}
          </button>
        </form>
      )}
    </>
  );
}
