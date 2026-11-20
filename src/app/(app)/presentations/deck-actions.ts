"use server";

import { z } from "zod";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import type { ActionResult } from "@/components/studio/editor";
import { DeckSlideSchema, SlideSchema, type DeckSlideInput, type SlideInput } from "./deck-schemas";

const BUCKET = "product-images";

const CreateSchema = z.object({
  title: z.string().trim().min(1, "Give the presentation a title"),
  slides: z.array(SlideSchema).min(3, "Pick at least one hamper or product"),
});

/** Saves a deck the browser has laid out, then opens it. */
export async function createPresentation(input: { title: string; slides: SlideInput[] }): Promise<ActionResult> {
  const parsed = CreateSchema.safeParse(input);
  if (!parsed.success) return { error: parsed.error.issues[0].message };

  const supabase = await createClient();
  const { data: deck, error } = await supabase
    .from("presentations")
    .insert({ title: parsed.data.title })
    .select("id")
    .single<{ id: string }>();
  if (error || !deck) return { error: describeError(error) };

  const { error: slideError } = await supabase
    .from("presentation_slides")
    .insert(parsed.data.slides.map((s, position) => ({ ...s, presentation_id: deck.id, position })));
  if (slideError) {
    await supabase.from("presentations").delete().eq("id", deck.id);
    return { error: describeError(slideError) };
  }

  revalidatePath("/presentations");
  redirect(`/presentations/${deck.id}`);
}

const DeckSaveSchema = z.object({
  title: z.string().trim().min(1, "The title can't be empty."),
  slides: z.array(DeckSlideSchema),
});

/**
 * Saves the deck page's draft in one go: the title, which slides remain,
 * their order, and any slides added. Returns the database ids given to the
 * added slides, keyed by their temporary ids.
 *
 * ponytail: several statements, not one transaction; move into an RPC if
 * two people editing the same deck at once becomes a real case.
 */
export async function saveDeck(
  presentationId: string,
  input: { title: string; slides: DeckSlideInput[] },
): Promise<ActionResult & { ids?: Record<string, string> }> {
  const parsed = DeckSaveSchema.safeParse(input);
  if (!parsed.success) return { error: parsed.error.issues[0].message };
  const { title, slides } = parsed.data;

  try {
    const supabase = await createClient();
    const { data: existing, error: loadError } = await supabase
      .from("presentation_slides")
      .select("id, position")
      .eq("presentation_id", presentationId)
      .overrideTypes<{ id: string; position: number }[], { merge: false }>();
    if (loadError) return { error: describeError(loadError) };

    const current = new Map((existing ?? []).map((r) => [r.id, r.position]));
    const kept = slides.flatMap((s) => ("id" in s ? [s.id] : []));
    if (kept.some((id) => !current.has(id))) {
      return { error: "Some slides were changed somewhere else. Reload the page and make your changes again." };
    }

    const removed = [...current.keys()].filter((id) => !kept.includes(id));
    if (removed.length) {
      const { error } = await supabase.from("presentation_slides").delete().in("id", removed);
      if (error) return { error: describeError(error) };
    }

    const added = slides.flatMap((s, position) => ("tempId" in s ? [{ s, position }] : []));
    const ids: Record<string, string> = {};
    if (added.length) {
      const { data: inserted, error } = await supabase
        .from("presentation_slides")
        .insert(
          added.map(({ s, position }) => ({
            presentation_id: presentationId,
            position,
            kind: s.kind,
            hamper_id: s.hamper_id,
            product_id: s.product_id,
            canvas: s.canvas,
          })),
        )
        .select("id, position")
        .overrideTypes<{ id: string; position: number }[], { merge: false }>();
      if (error || !inserted) return { error: describeError(error) };
      for (const row of inserted) {
        const match = added.find((a) => a.position === row.position);
        if (match) ids[match.s.tempId] = row.id;
      }
    }

    const moves = slides.flatMap((s, position) => ("id" in s && current.get(s.id) !== position ? [{ id: s.id, position }] : []));
    const results = await Promise.all(
      moves.map((m) => supabase.from("presentation_slides").update({ position: m.position }).eq("id", m.id)),
    );
    const failed = results.find((r) => r.error);
    if (failed?.error) return { error: describeError(failed.error) };

    const { error: titleError } = await supabase.from("presentations").update({ title }).eq("id", presentationId);
    if (titleError) return { error: describeError(titleError) };

    return { ok: true, ids };
  } catch (err) {
    return { error: describeError(err) };
  }
}

export async function deletePresentation(presentationId: string): Promise<ActionResult> {
  const supabase = await createClient();
  const { error } = await supabase.from("presentations").delete().eq("id", presentationId);
  if (error) return { error: describeError(error) };

  // Best effort: uploaded backgrounds for this deck.
  const folder = `presentations/${presentationId}`;
  const { data: files } = await supabase.storage.from(BUCKET).list(folder);
  if (files?.length) await supabase.storage.from(BUCKET).remove(files.map((f) => `${folder}/${f.name}`));

  revalidatePath("/presentations");
  redirect("/presentations");
}
