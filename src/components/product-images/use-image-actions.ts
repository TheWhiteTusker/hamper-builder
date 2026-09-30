import { useEffect, useState, useTransition } from "react";
import { addExternalProductImage } from "@/app/(app)/products/image-actions";
import { uploadProductImage } from "./upload-product-image";
import {
  setPrimaryProductImage,
  updateProductImageColor,
  deleteProductImage,
} from "@/app/(app)/products/image-edit-actions";
import type { ProductImage } from "@/lib/types";

export type Feedback = { error?: string; success?: string };

/** "General" is how the pickers say "no colour". */
const colorOrNull = (color: string) => (color === "General" ? null : color);

/** A product's photos, and the server actions that change them. */
export function useImageActions(
  productId: string,
  initialImages: ProductImage[],
  onImagesChange?: (images: ProductImage[]) => void,
) {
  const [images, setImages] = useState<ProductImage[]>(initialImages);
  useEffect(() => onImagesChange?.(images), [images, onImagesChange]);
  const [feedback, setFeedback] = useState<Feedback>({});
  const [isPending, startTransition] = useTransition();

  // A new primary photo takes the badge from the old one of the same product
  // (the list can hold sibling colour products' photos too).
  const withPrimary = (list: ProductImage[], img: ProductImage) =>
    list.map((i) => (i.product_id === img.product_id ? { ...i, is_primary: i.id === img.id } : i));
  const withAdded = (list: ProductImage[], img: ProductImage) =>
    img.is_primary ? withPrimary([...list, img], img) : [...list, img];
  const ownerOf = (imageId: string) => images.find((i) => i.id === imageId)?.product_id ?? productId;

  function upload(files: File[], color: string, asPrimary: boolean) {
    if (!files.length) return;
    setFeedback({});
    startTransition(async () => {
      let next = images;
      for (const [i, file] of files.entries()) {
        const res = await uploadProductImage(file, {
          productId,
          color: colorOrNull(color),
          isPrimary: asPrimary && i === 0,
        });
        if (res.error) {
          setImages(next);
          return setFeedback({ error: res.error });
        }
        if (res.image) next = withAdded(next, res.image);
      }
      setImages(next);
      setFeedback({ success: "Image(s) uploaded successfully!" });
    });
  }

  function addUrl(url: string, color: string, asPrimary: boolean, onAdded: () => void) {
    if (!url.trim()) return;
    setFeedback({});
    startTransition(async () => {
      const res = await addExternalProductImage(productId, url.trim(), colorOrNull(color), asPrimary);
      if (res.error) return setFeedback({ error: res.error });
      if (!res.image) return;
      setImages(withAdded(images, res.image));
      onAdded();
      setFeedback({ success: "Image URL added successfully!" });
    });
  }

  function setPrimary(imageId: string) {
    startTransition(async () => {
      const res = await setPrimaryProductImage(imageId, ownerOf(imageId));
      if (res.error) return setFeedback({ error: res.error });
      if (res.image) setImages((prev) => withPrimary(prev, res.image!));
      setFeedback({ success: "Primary cover image updated." });
    });
  }

  function changeColor(imageId: string, newColor: string) {
    startTransition(async () => {
      const res = await updateProductImageColor(imageId, colorOrNull(newColor));
      if (res.error) return setFeedback({ error: res.error });
      if (res.image) setImages((prev) => prev.map((img) => (img.id === imageId ? res.image! : img)));
    });
  }

  function remove(imageId: string) {
    if (!confirm("Move this image to the Bin? It will stay in the Bin for 30 days and can be restored anytime.")) return;
    startTransition(async () => {
      const res = await deleteProductImage(imageId, ownerOf(imageId));
      if (res.error) return setFeedback({ error: res.error });
      setImages((prev) => prev.filter((img) => img.id !== imageId));
      setFeedback({ success: "Image moved to Bin. You can restore it from the Bin within 30 days." });
    });
  }

  return { images, feedback, isPending, upload, addUrl, setPrimary, changeColor, remove };
}
