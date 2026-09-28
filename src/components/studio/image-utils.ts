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
