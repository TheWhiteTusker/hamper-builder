import { notFound } from "next/navigation";
import Link from "@/components/link";
import { requireUser, canManage } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { ImagePreview } from "@/components/image-preview";
import { round2 } from "@/lib/pricing";
import type { HamperSummary } from "@/lib/types";
import { HamperLines, type PreviewItem } from "./_view/hamper-lines";
import { HamperTotals } from "./_view/hamper-totals";

/** A saved hamper, read-only, laid out like a product's page; Edit opens the builder. */
export default async function HamperPage({ params }: { params: Promise<{ code: string }> }) {
  const { code } = await params;
  const { supabase, profile } = await requireUser();

  // The summary view carries the saved totals, so nothing is recomputed here.
  const { data: hamper } = await supabase
    .from("hamper_summary")
    .select("*")
    .eq("code", decodeURIComponent(code))
    .maybeSingle<HamperSummary>();
  if (!hamper) notFound();

  const { data: items } = await supabase
    .from("hamper_items")
    .select("*, products(image_url)")
    .eq("hamper_id", hamper.id)
    .order("line_no")
    .returns<PreviewItem[]>();

  const fields: [string, string][] = [
    ["Hamper code", hamper.code],
    ["Collection", hamper.collection ?? "—"],
    ["Status", hamper.status],
    ["No. of items", String(round2(hamper.number_of_items))],
  ];

  return (
    <>
      <PageHeader title={hamper.name} subtitle={hamper.code}>
        <span className="badge">{hamper.status}</span>
        {canManage(profile.role) && (
          <Link href={`/hampers/${encodeURIComponent(hamper.code)}/edit`} className="btn-primary">
            Edit
          </Link>
        )}
      </PageHeader>

      <div className="space-y-6">
        <div className="card flex flex-wrap gap-5 p-5">
          {hamper.image_url && (
            <ImagePreview
              src={hamper.image_url}
              alt={`${hamper.name} image`}
              sizes="128px"
              className="h-32 w-32 shrink-0 rounded-lg border border-slate-200 bg-slate-100"
            />
          )}
          <div className="grid flex-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {fields.map(([label, value]) => (
              <div key={label}>
                <div className="label">{label}</div>
                <div className="mt-1 text-sm text-(--color-ink)">{value}</div>
              </div>
            ))}
            {hamper.notes && (
              <div className="sm:col-span-2 lg:col-span-4">
                <div className="label">Notes</div>
                <p className="mt-1 whitespace-pre-line text-sm text-(--color-ink)">{hamper.notes}</p>
              </div>
            )}
          </div>
        </div>

        <HamperLines items={items ?? []} />
        <HamperTotals h={hamper} />
      </div>
    </>
  );
}
