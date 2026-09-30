import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { UploadTicket } from "@/components/studio/editor";

const BUCKET = "product-images";
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "webp", "gif"]);

/**
 * A one-time URL the browser uploads the image to directly, so the file never
 * passes through the server (whose body limits broke large uploads).
 */
export async function signedImageUpload(folder: string, ext: string): Promise<UploadTicket> {
  try {
    // A page loaded before this change still sends FormData: ask for a reload.
    if (typeof ext !== "string") return { error: "The app was updated. Please reload the page and try again." };
    ext = ext.toLowerCase();
    if (!IMAGE_EXTS.has(ext)) return { error: "That file is not a supported image (PNG, JPG, WebP or GIF)." };

    const supabase = await createClient();
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return { error: "You must be signed in to upload images." };

    const path = `${folder}/asset-${Date.now()}-${Math.random().toString(36).slice(2, 6)}.${ext}`;
    const { data, error } = await supabase.storage.from(BUCKET).createSignedUploadUrl(path);
    if (error) return { error: `Storage upload failed: ${describeError(error)}` };
    return { path, token: data.token, url: supabase.storage.from(BUCKET).getPublicUrl(path).data.publicUrl };
  } catch (err) {
    return { error: describeError(err) };
  }
}
