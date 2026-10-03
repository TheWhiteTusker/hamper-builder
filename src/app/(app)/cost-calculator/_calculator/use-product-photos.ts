import { useCallback, useState } from "react";
import { productPhotosChanged } from "@/app/(app)/products/image-actions";
import { uploadProductImage } from "@/components/product-images/upload-product-image";
import type { ProductImage } from "@/lib/types";

type Pending = { key: string; color: string; file: File; url: string };
type UploadStatus = { color: string; busy?: boolean; error?: string; ok?: string };

/** Uploads files for one colour. Returns what landed and the first error. */
async function uploadFiles(
  productId: string,
  colName: string,
  files: File[],
  existing: number,
  onEach: (i: number) => void,
) {
  const added: ProductImage[] = [];
  for (const [i, file] of files.entries()) {
    onEach(i);
    const res = await uploadProductImage(file, {
      productId,
      color: colName,
      // First photo of a product with none becomes its cover
      isPrimary: existing + added.length === 0 && i === 0,
    });
    if (res.error || !res.image) return { added, error: res.error ?? "Upload failed." };
    added.push(res.image);
  }
  return { added, error: "" };
}

export type ProductPhotos = ReturnType<typeof useProductPhotos>;

/**
 * Photos per colour row. Before the product's first save a photo row has no
 * product id, so picked photos wait in the browser and upload after saving.
 */
export function useProductPhotos(
  initialImages: ProductImage[],
  selectedProductId: string,
  initialVariantIds: Record<string, string> = {},
) {
  const [images, setImages] = useState<ProductImage[]>(initialImages);
  // Bumped after a row upload so the gallery remounts with the new photos.
  const [photoVersion, setPhotoVersion] = useState(0);
  // Colour -> product id once a multi-colour save has split the product.
  const [variantIds, setVariantIds] = useState<Record<string, string>>(initialVariantIds);
  const [pending, setPending] = useState<Pending[]>([]);
  const [upload, setUpload] = useState<UploadStatus | null>(null);

  // The gallery holds every colour's photos, sibling products' included.
  const handleGalleryChange = useCallback((gallery: ProductImage[]) => setImages(gallery), []);

  // Status shows on the row itself so a failure is never silent.
  async function uploadForColor(colName: string, files: File[]) {
    if (!selectedProductId) {
      setPending((prev) => [
        ...prev,
        ...files.map((file) => ({ key: crypto.randomUUID(), color: colName, file, url: URL.createObjectURL(file) })),
      ]);
      setUpload({ color: colName, ok: `${files.length} will upload on save` });
      return;
    }

    setUpload({ color: colName, busy: true });
    const pid = variantIds[colName] ?? selectedProductId;
    const existing = images.filter((img) => img.product_id === pid).length;
    const { added, error } = await uploadFiles(pid, colName, files, existing, (i) =>
      setUpload({ color: colName, busy: true, ok: `Uploading ${i + 1} of ${files.length}…` }),
    );
    if (added.length) {
      setImages((prev) => [...prev, ...added]);
      setPhotoVersion((v) => v + 1);
      // Once per batch; a failure here only means other pages refresh a little later.
      productPhotosChanged().catch(() => {});
    }
    setUpload(error ? { color: colName, error } : { color: colName, ok: `${added.length} added` });
  }

  function removePending(key: string) {
    setPending((prev) => {
      const gone = prev.find((p) => p.key === key);
      if (gone) URL.revokeObjectURL(gone.url);
      return prev.filter((p) => p.key !== key);
    });
  }

  /** After a save: remember each colour's product and upload the queued photos. */
  async function afterSave(ids: Record<string, string>, productId: string | undefined) {
    setVariantIds(ids);
    if (!productId || !pending.length) return { message: "", failed: false };

    const added: ProductImage[] = [];
    const failed: string[] = [];
    let done = 0;
    for (const colName of new Set(pending.map((p) => p.color))) {
      const pid = ids[colName] ?? productId;
      const files = pending.filter((p) => p.color === colName).map((p) => p.file);
      const existing = [...images, ...added].filter((img) => img.product_id === pid).length;
      // No productPhotosChanged() here: the save already refreshed, and the page moves on next.
      const r = await uploadFiles(pid, colName, files, existing, () =>
        setUpload({ color: colName, busy: true, ok: `Uploading photo ${++done} of ${pending.length}…` }),
      );
      added.push(...r.added);
      if (r.error) failed.push(`${colName}: ${r.error}`);
    }
    pending.forEach((p) => URL.revokeObjectURL(p.url));
    setPending([]);
    setUpload(null);
    setImages((prev) => [...prev, ...added]);
    setPhotoVersion((v) => v + 1);
    return {
      failed: failed.length > 0,
      message: failed.length
        ? ` ${added.length} photo(s) uploaded; failed — ${failed.join("; ")}`
        : ` ${added.length} photo(s) uploaded.`,
    };
  }

  return { images, photoVersion, pending, upload, handleGalleryChange, uploadForColor, removePending, afterSave };
}
