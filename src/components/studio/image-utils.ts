import { createClient } from "@/lib/supabase/client";
import type { ActionResult, UploadTicket } from "./editor";

/** Every upload gets a fresh random path and is never overwritten, so browsers may keep it for a year. */
export const IMMUTABLE = "31536000";

/** Uploads straight to storage via a signed URL from `prepare`, so the file skips the server's body limits. */
export async function uploadDirect(
  prepare: (ext: string) => Promise<UploadTicket>,
  file: File,
): Promise<ActionResult & { path?: string }> {
  if (!file.type.startsWith("image/")) return { error: "That file is not an image." };
  const ext = file.name.split(".").pop()?.toLowerCase() || file.type.split("/")[1];
  const ticket = await prepare(ext);
  if (ticket.error || !ticket.path || !ticket.token) return { error: ticket.error ?? "Upload failed." };
  const { error } = await createClient()
    .storage.from("product-images")
    .uploadToSignedUrl(ticket.path, ticket.token, file, { contentType: file.type, cacheControl: IMMUTABLE });
  if (error) return { error: `Storage upload failed: ${error.message}` };
  return { ok: true, url: ticket.url, path: ticket.path };
}

/**
 * Client-side image optimization for the studio canvas.
 * Scales large camera/phone photos down to max 2000px and compresses to ~300-500KB
 * before uploading, preventing network timeouts and massive payload transfers.
 */
export async function optimizeImageForUpload(file: File, maxDimension = 2000): Promise<File> {
  // If not an image, return as is
  if (!file.type.startsWith("image/") || file.type === "image/svg+xml") {
    return file;
  }

  // If already small (< 1MB), return as is
  if (file.size <= 1024 * 1024) {
    return file;
  }

  try {
    return await new Promise<File>((resolve) => {
      const img = new Image();
      const objectUrl = URL.createObjectURL(file);

      img.onload = () => {
        URL.revokeObjectURL(objectUrl);
        const { naturalWidth: w, naturalHeight: h } = img;
        const scale = Math.min(1, maxDimension / Math.max(w, h));

        // If dimensions are within max and size is under 2MB, keep original
        if (scale === 1 && file.size < 2 * 1024 * 1024) {
          resolve(file);
          return;
        }

        const targetW = Math.round(w * scale);
        const targetH = Math.round(h * scale);

        const canvas = document.createElement("canvas");
        canvas.width = targetW;
        canvas.height = targetH;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          resolve(file);
          return;
        }

        ctx.drawImage(img, 0, 0, targetW, targetH);

        const isPng = file.type === "image/png" || file.name.toLowerCase().endsWith(".png");
        const mimeType = isPng ? "image/png" : "image/jpeg";
        const quality = isPng ? undefined : 0.85;

        canvas.toBlob(
          (blob) => {
            if (!blob || blob.size >= file.size) {
              resolve(file);
              return;
            }
            const ext = isPng ? "png" : "jpg";
            const name = file.name.replace(/\.[^.]+$/, "") + `.${ext}`;
            resolve(new File([blob], name, { type: mimeType }));
          },
          mimeType,
          quality,
        );
      };

      img.onerror = () => {
        URL.revokeObjectURL(objectUrl);
        resolve(file);
      };

      img.src = objectUrl;
    });
  } catch {
    return file;
  }
}

/** `file` scaled to fit within `max` px and re-encoded as WebP: keeps transparency, far smaller than PNG. */
export async function toWebp(file: File, max: number, quality: number): Promise<File> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, max / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext("2d")?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const blob = await new Promise<Blob | null>((done) => canvas.toBlob(done, "image/webp", quality));
  if (!blob) throw new Error("Could not process that image.");
  // A browser that cannot encode WebP hands back a PNG instead.
  return new File([blob], `photo.${blob.type.split("/")[1]}`, { type: blob.type });
}
