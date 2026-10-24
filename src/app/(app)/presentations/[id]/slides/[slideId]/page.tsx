import { redirect } from "next/navigation";
import { requireUser } from "@/lib/supabase/server";
import { parseCanvas } from "@/lib/hamper-canvas-schema";
import { CanvasEditor } from "@/components/studio/canvas-editor";
import { loadCatalog } from "@/app/(app)/hampers/data";
import { saveSlide, uploadPresentationAsset } from "../../../slide-actions";

export default async function SlideEditorPage({ params }: { params: Promise<{ id: string; slideId: string }> }) {
  const { id, slideId } = await params;
  const { supabase } = await requireUser();

  const [{ data: deck }, { data: slides }, catalog] = await Promise.all([
    supabase.from("presentations").select("id, title").eq("id", id).maybeSingle<{ id: string; title: string }>(),
    supabase
      .from("presentation_slides")
      .select("id, canvas, hamper_id, product_id")
      .eq("presentation_id", id)
      .order("position")
      .returns<{ id: string; canvas: unknown; hamper_id: string | null; product_id: string | null }[]>(),
    loadCatalog(supabase),
  ]);

  const all = slides ?? [];
  const index = all.findIndex((s) => s.id === slideId);
  // A deleted slide (or deck) goes back to where it lived rather than a 404,
  // e.g. when a refresh lands on the slide that was just removed.
  if (!deck) redirect("/presentations");
  if (index < 0) redirect(`/presentations/${id}`);
  const slide = all[index];

  // Offer the slide's own hamper contents (or product) first in the Products panel.
  const related = slide.hamper_id
    ? (
        (
          await supabase
            .from("hamper_items")
            .select("product_id")
            .eq("hamper_id", slide.hamper_id)
            .returns<{ product_id: string | null }[]>()
        ).data ?? []
      ).flatMap((i) => (i.product_id ? [i.product_id] : []))
    : slide.product_id
      ? [slide.product_id]
      : [];

  return (
    <CanvasEditor
      title={deck.title}
      subtitle={`Slide ${index + 1} of ${all.length}`}
      backHref={`/presentations/${id}`}
      downloadName={`${deck.title} - slide ${index + 1}`}
      onSave={saveSlide.bind(null, id, slideId)}
      onUpload={uploadPresentationAsset.bind(null, id)}
      saveImage={false}
      savedMessage="Slide saved."
      initial={parseCanvas(slide.canvas)}
      hamperProductIds={[...new Set(related)]}
      products={catalog.products.map((p) => ({ id: p.id, code: p.code, name: p.name, image_url: p.image_url ?? null }))}
      pages={all.map((s) => ({ id: s.id, href: `/presentations/${id}/slides/${s.id}`, canvas: parseCanvas(s.canvas) }))}
      pageId={slideId}
    />
  );
}
