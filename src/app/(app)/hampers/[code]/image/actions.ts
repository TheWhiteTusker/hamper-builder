"use server";

import { revalidatePath } from "next/cache";
import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { CanvasSchema } from "@/lib/hamper-canvas";
import type { ActionResult } from "@/components/studio/editor";

const BUCKET = "product-images";

/** "…/object/public/product-images/hampers/x/y.png?v=1" -> "hampers/x/y.png" */
const storagePathOf = (url: string | null) =>
  url?.split(`/${BUCKET}/`)[1]?.split("?")[0] ?? null;

async function upload(folder: string, file: File, name: string) {
  const supabase = await createClient();
  const {
    data: { user },
    error: userError,
  } = await supabase.auth.getUser();

  if (userError || !user) {
    throw new Error("You must be signed in to upload images.");
  }

  const path = `hampers/${folder}/${name}`;
  const ext = name.split(".").pop()?.toLowerCase() || "jpg";
  const contentType =
    file.type ||
    (ext === "png"
      ? "image/png"
      : ext === "webp"
        ? "image/webp"
        : ext === "gif"
          ? "image/gif"
          : "image/jpeg");

  const { error } = await supabase.storage
    .from(BUCKET)
    .upload(path, Buffer.from(await file.arrayBuffer()), {
      contentType,
      upsert: true,
    });
  if (error) throw new Error(`Storage upload failed: ${describeError(error)}`);
  return { supabase, path, url: supabase.storage.from(BUCKET).getPublicUrl(path).data.publicUrl };
}

/** Saves the layout and the PNG rendered from it, replacing the previous PNG. Bind the hamper id. */
export async function saveHamperCanvas(hamperId: string, formData: FormData): Promise<ActionResult> {
  try {
    const png = formData.get("png");
    if (!hamperId) return { error: "Missing hamper." };
    if (!(png instanceof File) || png.size === 0) return { error: "The image could not be rendered." };

    let raw: unknown;
    try {
      raw = JSON.parse(String(formData.get("canvas") ?? ""));
    } catch {
      return { error: "Could not read the design." };
    }
    const canvas = CanvasSchema.safeParse(raw);
    if (!canvas.success) return { error: `Invalid design: ${canvas.error.issues[0].message}` };

    const { supabase, path, url } = await upload(hamperId, png, `cover-${Date.now()}.png`);

    const { data: before } = await supabase
      .from("hampers")
      .select("code, image_url")
      .eq("id", hamperId)
      .maybeSingle<{ code: string; image_url: string | null }>();

    // RLS turns a non-manager's update into zero rows rather than an error.
    const { data: updated, error } = await supabase
      .from("hampers")
      .update({ canvas: canvas.data, image_url: url })
      .eq("id", hamperId)
      .select("id");

    if (error || !updated?.length) {
      await supabase.storage.from(BUCKET).remove([path]);
      return { error: error ? describeError(error) : "You don't have permission to edit this hamper." };
    }

    // Only delete a PNG in this hamper's own folder: a duplicated hamper
    // still points at the original's file until it is saved once.
    const oldPath = storagePathOf(before?.image_url ?? null);
    if (oldPath?.startsWith(`hampers/${hamperId}/`)) {
      await supabase.storage.from(BUCKET).remove([oldPath]);
    }

    revalidatePath("/hampers");
    if (before) revalidatePath(`/hampers/${encodeURIComponent(before.code)}`);
    return { ok: true, url };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

// ponytail: replaced backgrounds and removed uploads stay in storage; clean up on save if the bucket grows.
export async function uploadHamperBackground(hamperId: string, formData: FormData): Promise<ActionResult> {
  try {
    const file = formData.get("file");
    if (!hamperId) return { error: "Missing hamper." };
    if (!(file instanceof File) || file.size === 0) return { error: "Please choose an image." };
    if (!file.type.startsWith("image/")) return { error: "That file is not an image." };

    const ext = file.name.split(".").pop()?.toLowerCase() || "png";
    const { url } = await upload(hamperId, file, `asset-${Date.now()}-${Math.random().toString(36).slice(2, 6)}.${ext}`);
    return { ok: true, url };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}
