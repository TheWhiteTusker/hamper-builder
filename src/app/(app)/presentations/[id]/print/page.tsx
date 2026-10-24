import { redirect } from "next/navigation";
import { requireUser } from "@/lib/supabase/server";
import { parseCanvas } from "@/lib/hamper-canvas-schema";
import { PrintDeck } from "./print-deck";

export default async function PrintPresentationPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const { supabase } = await requireUser();

  const [{ data: deck }, { data: slides }] = await Promise.all([
    supabase.from("presentations").select("id, title").eq("id", id).maybeSingle<{ id: string; title: string }>(),
    supabase
      .from("presentation_slides")
      .select("id, canvas")
      .eq("presentation_id", id)
      .order("position")
      .returns<{ id: string; canvas: unknown }[]>(),
  ]);
  if (!deck) redirect("/presentations"); // deleted, or a stale link

  return <PrintDeck deck={deck} slides={(slides ?? []).map((s) => ({ id: s.id, canvas: parseCanvas(s.canvas) }))} />;
}
