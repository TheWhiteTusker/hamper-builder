import { prepareProductImageUpload, saveUploadedProductImage } from "@/app/(app)/products/image-actions";
import type { ImageActionResult } from "@/app/(app)/products/image-types";
import { optimizeImageForUpload, uploadDirect } from "@/components/studio/image-utils";

/**
 * Shrinks the photo, uploads it straight to storage (skipping the server's
 * body limits), then records it on the product.
 */
export async function uploadProductImage(
  file: File,
  opts: { productId: string; color?: string | null; isPrimary?: boolean; caption?: string | null },
): Promise<ImageActionResult> {
  try {
    const up = await uploadDirect(
      (ext) => prepareProductImageUpload(opts.productId, ext),
      await optimizeImageForUpload(file),
    );
    if (up.error || !up.path) return { error: up.error ?? "Upload failed." };
    return await saveUploadedProductImage({ ...opts, storagePath: up.path });
  } catch (e) {
    return { error: e instanceof Error ? e.message : "Upload failed." };
  }
}
