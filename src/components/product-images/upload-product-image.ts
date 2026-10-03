import { prepareProductImageUpload, saveUploadedProductImage } from "@/app/(app)/products/image-actions";
import type { ImageActionResult } from "@/app/(app)/products/image-types";
import { IMMUTABLE, optimizeImageForUpload, toWebp, uploadDirect } from "@/components/studio/image-utils";
import { thumbPath, viewPath } from "@/lib/product-images";
import { createClient } from "@/lib/supabase/client";

/**
 * Uploads the photo straight to storage (skipping the server's body limits),
 * then records it on the product. The original keeps its format (PNG stays
 * PNG) for presentations; two WebP copies alongside it are what product pages
 * show: a 2000px view and a 480px thumbnail.
 */
export async function uploadProductImage(
  file: File,
  opts: { productId: string; color?: string | null; isPrimary?: boolean; caption?: string | null },
): Promise<ImageActionResult> {
  try {
    const original = await optimizeImageForUpload(file);
    // A format the browser cannot decode (HEIC on Windows, say) gets no copies.
    const [view, thumb] = await Promise.all([
      toWebp(file, 2000, 0.85).catch(() => null),
      toWebp(file, 480, 0.8).catch(() => null),
    ]);

    let tokens: { thumbToken?: string; viewToken?: string } = {};
    const up = await uploadDirect(async (ext) => {
      const ticket = await prepareProductImageUpload(opts.productId, ext);
      tokens = ticket;
      return ticket;
    }, original);
    if (up.error || !up.path) return { error: up.error ?? "Upload failed." };

    // Not fatal: without a copy, product pages fall back to the original.
    const storage = createClient().storage.from("product-images");
    const copies: [string, string | undefined, File | null][] = [
      [thumbPath(up.path), tokens.thumbToken, thumb],
      [viewPath(up.path), tokens.viewToken, view],
    ];
    await Promise.all(
      copies.map(([path, token, copy]) =>
        token && copy ? storage.uploadToSignedUrl(path, token, copy, { contentType: copy.type, cacheControl: IMMUTABLE }) : null,
      ),
    );
    return await saveUploadedProductImage({ ...opts, storagePath: up.path });
  } catch (e) {
    return { error: e instanceof Error ? e.message : "Upload failed." };
  }
}
