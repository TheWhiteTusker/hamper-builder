import { notFound } from "next/navigation";
import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { RateSheetWizard } from "../rate-sheet-wizard";

const TITLES = { add: "Add new rates", edit: "Edit & add rates" };

export default async function RateSheetPage({ params }: { params: Promise<{ mode: string }> }) {
  await requireRole("admin");
  const { mode } = await params;
  if (mode !== "add" && mode !== "edit") notFound();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-bold">{TITLES[mode]} from Excel</h2>
        <Link href="/cost-calculator/master" className="text-sm font-semibold text-(--color-brand) underline">
          Back to the master
        </Link>
      </div>
      <RateSheetWizard mode={mode} />
    </div>
  );
}
