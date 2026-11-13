"use client";

import type { ProductColor } from "@/lib/product-code";
import { ImagePreview } from "@/components/image-preview";
import type { ProductDetails } from "./use-product-details";
import type { ProductPhotos } from "./use-product-photos";

/**
 * New product: one row per colour, tick the ones to create. Editing: just this
 * product's colour, plus any new colours added (each becomes a new product).
 */
export function ColorPhotoRows({
  details,
  photos,
  productColors,
}: {
  details: ProductDetails;
  photos: ProductPhotos;
  productColors: ProductColor[];
}) {
  const { upload } = photos;

  // Uploading to a colour also selects it.
  function handleUpload(colName: string, input: HTMLInputElement) {
    const files = Array.from(input.files ?? []);
    input.value = "";
    if (!files.length) return;
    if (!details.selectedColors.includes(colName)) details.toggleColor(colName);
    photos.uploadForColor(colName, files);
  }

  const { editing, ownColor, selectedColors } = details;
  const rows = editing
    ? selectedColors.flatMap((name) => productColors.filter((c) => c.name === name))
    : productColors;
  // Colours this product does not exist in yet.
  const addable = productColors.filter(
    (c) => !selectedColors.includes(c.name) && !details.siblingColors.includes(c.name),
  );

  return (
    <>
    <ul className="mt-2 divide-y divide-[var(--color-border)] rounded-lg border border-[var(--color-border)]">
      {rows.map((col) => {
        const colImages = photos.images.filter(
          (img) => img.color === col.name || img.color_code === col.code,
        );
        const colPending = photos.pending.filter((p) => p.color === col.name);
        const status = upload?.color === col.name ? upload : null;
        return (
          <li key={col.name} className="flex flex-wrap items-center gap-3 px-3 py-2">
            <label className="flex min-w-[190px] cursor-pointer items-center gap-2 text-xs font-medium text-[var(--color-ink)]">
              {!editing && (
                <input
                  type="checkbox"
                  checked={selectedColors.includes(col.name)}
                  onChange={() => details.toggleColor(col.name)}
                  className="h-4 w-4 rounded text-[var(--color-brand)] focus:ring-[var(--color-brand)]"
                />
              )}
              <span
                className="inline-block h-3.5 w-3.5 rounded-full border border-black/20"
                style={{ backgroundColor: col.hex }}
              />
              {col.name} ({col.code})
              {editing &&
                (col.name === ownColor ? (
                  <span className="badge">This product</span>
                ) : (
                  <span className="text-[11px] font-normal text-amber-700">New · created on save</span>
                ))}
            </label>

            <div className="flex flex-1 items-center gap-1.5">
              {colImages.length === 0 && colPending.length === 0 ? (
                <span className="text-[11px] text-[var(--color-muted)]">No photos</span>
              ) : (
                colImages.slice(0, 5).map((img) => (
                  <ImagePreview
                    key={img.id}
                    src={img.url}
                    alt={`${details.name || "Product"} — ${col.name}`}
                    sizes="40px"
                    className="h-10 w-10 rounded-md border border-slate-200 bg-slate-100"
                  />
                ))
              )}
              {colPending.map((p) => (
                <span
                  key={p.key}
                  title="Uploads when you save"
                  className="relative h-10 w-10 overflow-hidden rounded-md border border-dashed border-amber-400"
                >
                  {/* eslint-disable-next-line @next/next/no-img-element -- local blob preview */}
                  <img src={p.url} alt={p.file.name} className="h-full w-full object-cover opacity-80" />
                  <button
                    type="button"
                    onClick={() => photos.removePending(p.key)}
                    title="Remove"
                    className="absolute right-0 top-0 flex h-4 w-4 items-center justify-center rounded-bl bg-white/90 text-[10px] leading-none text-red-600"
                  >
                    &times;
                  </button>
                </span>
              ))}
              {colImages.length > 5 && (
                <span className="text-[11px] text-[var(--color-muted)]">+{colImages.length - 5}</span>
              )}
            </div>

            {status?.busy && <span className="text-[11px] text-[var(--color-muted)]">Uploading…</span>}
            {status?.error && <span className="text-[11px] font-medium text-red-600">{status.error}</span>}
            {status?.ok && <span className="text-[11px] font-medium text-emerald-700">{status.ok}</span>}

            <label
              title={
                details.selectedProductId
                  ? `Upload ${col.name} photos`
                  : `Add ${col.name} photos; they upload when you save`
              }
              className={`rounded-md border px-2.5 py-1 text-[11px] font-semibold transition-colors ${
                !upload?.busy
                  ? "cursor-pointer border-[var(--color-brand)] text-[var(--color-brand)] hover:bg-emerald-50"
                  : "cursor-not-allowed border-slate-200 text-slate-400"
              }`}
            >
              📷 Upload
              <input
                type="file"
                accept="image/*"
                multiple
                hidden
                disabled={upload?.busy}
                onChange={(e) => handleUpload(col.name, e.currentTarget)}
              />
            </label>
            {editing && col.name !== ownColor && (
              <button
                type="button"
                onClick={() => details.toggleColor(col.name)}
                className="text-[11px] font-semibold text-red-700 hover:underline"
              >
                Remove
              </button>
            )}
          </li>
        );
      })}
    </ul>
    {editing && addable.length > 0 && (
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
        <label htmlFor="add-color" className="font-semibold text-[var(--color-ink)]">
          + Add another colour
        </label>
        <select
          id="add-color"
          value=""
          onChange={(e) => e.target.value && details.toggleColor(e.target.value)}
          className="select py-1 text-xs"
        >
          <option value="">Choose…</option>
          {addable.map((c) => (
            <option key={c.name} value={c.name}>
              {c.name} ({c.code})
            </option>
          ))}
        </select>
        <span className="text-[var(--color-muted)]">Saved as a new product, starting from this costing.</span>
      </div>
    )}
    </>
  );
}
