"use server";

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { CanvasSchema } from "@/lib/hamper-canvas";
import { signedImageUpload } from "@/lib/signed-upload";
import type { ActionResult, UploadTicket } from "@/components/studio/editor";

type Supabase = Awaited<ReturnType<typeof createClient>>;

async function touch(supabase: Supabase, presentationId: string) {
  // The trigger sets updated_at; any update fires it.
  await supabase.from("presentations").update({ updated_at: new Date().toISOString() }).eq("id", presentationId);
}

/* ---------------------------------------------------- photo editor actions */

/** Bind presentationId and slideId. */
export async function saveSlide(presentationId: string, slideId: string, formData: FormData): Promise<ActionResult> {
  try {
    let raw: unknown;
    try {
      raw = JSON.parse(String(formData.get("canvas") ?? ""));
    } catch {
      return { error: "Could not read the design." };
    }
    const canvas = CanvasSchema.safeParse(raw);
    if (!canvas.success) return { error: `Invalid design: ${canvas.error.issues[0].message}` };

    const supabase = await createClient();
    const { data, error } = await supabase
      .from("presentation_slides")
      .update({ canvas: canvas.data })
      .eq("id", slideId)
      .eq("presentation_id", presentationId)
      .select("id");
    if (error || !data?.length) return { error: error ? describeError(error) : "This slide no longer exists." };

    await touch(supabase, presentationId);
    return { ok: true };
  } catch (err) {
    return { error: describeError(err) };
  }
}

/** Bind presentationId. Uploads a background image for any slide of the deck. */
/** Bind presentationId. The browser then uploads the file to the returned URL. */
export async function uploadPresentationAsset(presentationId: string, ext: string): Promise<UploadTicket> {
  return signedImageUpload(`presentations/${presentationId}`, ext);
}
