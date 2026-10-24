import { redirect } from "next/navigation";
import { requireUser } from "@/lib/supabase/server";
import { parseCanvas } from "@/lib/hamper-canvas-schema";
import { loadPickerData } from "../data";
import { DeckOverview } from "./deck-overview";

export type SlideSummary = {
  id: string;
  kind: string;
  canvas: ReturnType<typeof parseCanvas>;
};

export default async function PresentationPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const { supabase } = await requireUser();

  const [{ data: deck }, { data: slides }, picker] = await Promise.all([
    supabase.from("presentations").select("id, title").eq("id", id).maybeSingle<{ id: string; title: string }>(),
    supabase
      .from("presentation_slides")
      .select("id, kind, canvas")
      .eq("presentation_id", id)
      .order("position")
      .returns<{ id: string; kind: string; canvas: unknown }[]>(),
    loadPickerData(supabase),
  ]);
  if (!deck) redirect("/presentations"); // deleted, or a stale link

  return (
    <DeckOverview
      deck={deck}
      slides={(slides ?? []).map((s) => ({ id: s.id, kind: s.kind, canvas: parseCanvas(s.canvas) }))}
      picker={picker}
    />
  );
}
