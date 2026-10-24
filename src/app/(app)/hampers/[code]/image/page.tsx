import { notFound } from "next/navigation";
import { requireRole } from "@/lib/supabase/server";
import { parseCanvas } from "@/lib/hamper-canvas-schema";
import type { Hamper } from "@/lib/types";
import { loadCatalog } from "../../data";
import { CanvasEditor } from "@/components/studio/canvas-editor";
import { saveHamperCanvas, uploadHamperBackground } from "./actions";

export default async function HamperImagePage({
  params,
}: {
  params: Promise<{ code: string }>;
}) {
  const { code } = await params;
  const { supabase } = await requireRole("admin", "manager");

  const { data: hamper } = await supabase
    .from("hampers")
    .select("*")
    .eq("code", decodeURIComponent(code))
    .maybeSingle<Hamper>();

  if (!hamper) notFound();

  const [{ data: items }, catalog] = await Promise.all([
    supabase
      .from("hamper_items")
      .select("product_id")
      .eq("hamper_id", hamper.id)
      .returns<{ product_id: string | null }[]>(),
    loadCatalog(supabase),
  ]);

  return (
    <CanvasEditor
      title={hamper.name}
      subtitle={hamper.code}
      backHref={`/hampers/${encodeURIComponent(hamper.code)}/edit`}
      downloadName={hamper.code}
      onSave={saveHamperCanvas.bind(null, hamper.id)}
      onUpload={uploadHamperBackground.bind(null, hamper.id)}
      savedMessage="Saved. The hamper image is updated."
      initial={parseCanvas(hamper.canvas)}
      hamperProductIds={[...new Set((items ?? []).flatMap((i) => (i.product_id ? [i.product_id] : [])))]}
      products={catalog.products.map((p) => ({
        id: p.id,
        code: p.code,
        name: p.name,
        image_url: p.image_url ?? null,
      }))}
    />
  );
}
