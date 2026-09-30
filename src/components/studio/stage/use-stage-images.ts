import { useState } from "react";
import type { Layer } from "@/lib/hamper-canvas";
import { removeBackground } from "@/lib/background-removal";
import { uploadProductImage } from "@/components/product-images/upload-product-image";
import { getBrandLogo } from "@/app/(app)/brand-actions";
import { baseLayer, type Editor, type UploadTicket } from "../editor";
import { loadImage } from "../render";
import { optimizeImageForUpload, uploadDirect } from "../image-utils";
import type { CanvasDoc } from "./use-canvas-doc";
import type { Toast } from "./use-stage-save";

/** Adding, swapping and uploading images, and cutting out product photos. */
export function useStageImages(
  doc: CanvasDoc,
  onUpload: (ext: string) => Promise<UploadTicket>,
  setToast: (t: Toast) => void,
) {
  const { canvas, add, addMany, patch, select, change } = doc;
  const W = canvas.width;
  const H = canvas.height;
  const [swapId, setSwapId] = useState<string | null>(null);
  const [uploads, setUploads] = useState<{ url: string; name: string }[]>([]);
  const [uploading, setUploading] = useState(false);
  const [removing, setRemoving] = useState(false);

  /** A new image layer at `at` (or the page centre), scaled to fit within `share` of the page. */
  const imageLayer = (img: HTMLImageElement, url: string, name: string, productId: string | null, at?: { x: number; y: number }, share = 0.45): Layer => {
    const k = Math.min(1, (Math.min(W, H) * share) / Math.max(img.naturalWidth, img.naturalHeight));
    const width = img.naturalWidth * k;
    const height = img.naturalHeight * k;
    const cx = at?.x ?? W / 2;
    const cy = at?.y ?? H / 2;
    return { ...baseLayer(cx - width / 2, cy - height / 2), kind: "image", name, product_id: productId, url, width, height, fit: "stretch" };
  };

  const addImage: Editor["addImage"] = async (url, name, at) => {
    try {
      add(imageLayer(await loadImage(url), url, name, null, at));
    } catch {
      setToast({ kind: "error", text: "That image could not be loaded." });
    }
  };

  const addLogo = async () => {
    const logo = await getBrandLogo();
    if (!logo) return setToast({ kind: "error", text: "Could not load the Lattice Lane logo." });
    try {
      add(imageLayer(await loadImage(logo.url), logo.url, "Logo", null, undefined, 0.3));
    } catch {
      setToast({ kind: "error", text: "Could not load the Lattice Lane logo." });
    }
  };

  const uploadImages: Editor["uploadImages"] = async (files) => {
    if (!files.length) return;
    setUploading(true);
    const added: Layer[] = [];
    const done: { url: string; name: string }[] = [];
    const failed: string[] = [];
    try {
      for (const [i, file] of files.entries()) {
        try {
          const res = await uploadDirect(onUpload, await optimizeImageForUpload(file));
          if (res.error || !res.url) {
            failed.push(`${file.name}: ${res.error ?? "upload failed"}`);
            continue;
          }
          done.push({ url: res.url, name: file.name });
          try {
            // Fan the new layers out a little so they don't sit exactly on top of each other.
            const offset = i * 40;
            const img = await loadImage(res.url);
            added.push(imageLayer(img, res.url, file.name, null, { x: W / 2 + offset, y: H / 2 + offset }));
          } catch {
            failed.push(`${file.name}: uploaded but could not be displayed`);
          }
        } catch (itemErr) {
          failed.push(`${file.name}: ${itemErr instanceof Error ? itemErr.message : "upload error"}`);
        }
      }
    } finally {
      setUploading(false);
    }
    setUploads((u) => [...done, ...u]);
    if (added.length) addMany(added);
    if (failed.length) setToast({ kind: "error", text: failed.join(" · ") });
    else if (done.length) setToast({ kind: "ok", text: `Added ${added.length} image${added.length === 1 ? "" : "s"}.` });
  };

  const placeImage: Editor["placeImage"] = async (product, url, at) => {
    try {
      const img = await loadImage(url);
      const current = canvas.layers.find((l) => l.id === swapId);
      if (current?.kind === "image") {
        // Keep the box width, position and rotation; follow the new aspect ratio.
        patch(current.id, {
          url,
          product_id: product.id,
          name: product.name,
          height: (current.width * img.naturalHeight) / img.naturalWidth,
        });
        setSwapId(null);
        select(current.id);
        return;
      }
      add(imageLayer(img, url, product.name, product.id, at));
    } catch {
      setToast({ kind: "error", text: "That image could not be loaded. If it's an external link, upload it to the product instead." });
    }
  };

  const uploadBackground = async (file: File) => {
    setUploading(true);
    try {
      const res = await uploadDirect(onUpload, await optimizeImageForUpload(file, 2500));
      if (res.error || !res.url) {
        setToast({ kind: "error", text: res.error ?? "Upload failed." });
        return;
      }
      await loadImage(res.url);
      change((c) => ({ ...c, background: { ...c.background, image_url: res.url! } }));
      setToast({ kind: "ok", text: "Background image updated." });
    } catch (err) {
      setToast({ kind: "error", text: err instanceof Error ? err.message : "Could not upload background image." });
    } finally {
      setUploading(false);
    }
  };

  const cutOut: Editor["cutOut"] = async (id, tolerance) => {
    const layer = canvas.layers.find((l) => l.id === id);
    if (layer?.kind !== "image" || !layer.product_id) return;
    setRemoving(true);
    try {
      const img = await loadImage(layer.url);
      // ponytail: capped at 2000px so a huge photo can't freeze the tab.
      const k = Math.min(1, 2000 / Math.max(img.naturalWidth, img.naturalHeight));
      const c = document.createElement("canvas");
      c.width = Math.round(img.naturalWidth * k);
      c.height = Math.round(img.naturalHeight * k);
      const ctx = c.getContext("2d")!;
      ctx.drawImage(img, 0, 0, c.width, c.height);
      const pixels = ctx.getImageData(0, 0, c.width, c.height);
      removeBackground(pixels.data, c.width, c.height, tolerance);
      ctx.putImageData(pixels, 0, 0);
      const blob = await new Promise<Blob | null>((r) => c.toBlob(r, "image/png"));
      if (!blob) throw new Error("render failed");

      const res = await uploadProductImage(new File([blob], "cutout.png", { type: "image/png" }), {
        productId: layer.product_id,
        caption: "Background removed",
      });
      if (res.error || !res.image) return setToast({ kind: "error", text: res.error ?? "Upload failed." });
      patch(id, { url: res.image.url });
      setToast({ kind: "ok", text: "Background removed. The cut-out is saved to the product's images." });
    } catch {
      setToast({ kind: "error", text: "Could not process that image." });
    } finally {
      setRemoving(false);
    }
  };

  return { swapId, setSwapId, uploads, uploading, removing, addImage, addLogo, uploadImages, placeImage, uploadBackground, cutOut };
}
