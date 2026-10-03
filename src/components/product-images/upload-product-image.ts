import { prepareProductImageUpload, saveUploadedProductImage } from "@/app/(app)/products/image-actions";
import type { ImageActionResult } from "@/app/(app)/products/image-types";
import { toWebp, uploadDirect } from "@/components/studio/image-utils";
import { thumbPath } from "@/lib/product-images";
import { createClient } from "@/lib/supabase/client";

/**
 * Shrinks the photo to WebP, uploads it and a small thumbnail straight to
 * storage (skipping the server's body limits), then records it on the product.
 * Lists and galleries show the thumbnail; the preview opens the full photo.
 */
export async function uploadProductImage(
  file: File,
  opts: { productId: string; color?: string | null; isPrimary?: boolean; caption?: string | null },
): Promise<ImageActionResult> {
  try {
    // A format the browser cannot decode (HEIC on Windows, say) goes up as-is.
    const webp = await toWebp(file, 2000, 0.85).catch(() => file);
    const full = webp.size < file.size ? webp : file;
    const thumb = await toWebp(file, 480, 0.8).catch(() => null);

    let thumbToken: string | undefined;
    const up = await uploadDirect(async (ext) => {
      const ticket = await prepareProductImageUpload(opts.productId, ext);
      thumbToken = ticket.thumbToken;
      return ticket;
    }, full);
    if (up.error || !up.path) return { error: up.error ?? "Upload failed." };

    // Not fatal: without a thumbnail, lists fall back to the full photo.
    if (thumb && thumbToken) {
      await createClient()
        .storage.from("product-images")
        .uploadToSignedUrl(thumbPath(up.path), thumbToken, thumb, { contentType: thumb.type });
    }
    return await saveUploadedProductImage({ ...opts, storagePath: up.path });
  } catch (e) {
    return { error: e instanceof Error ? e.message : "Upload failed." };
  }
}
