import { notFound } from "next/navigation";
import { requireRole } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { ProductSheetWizard } from "../product-sheet-wizard";

const TITLES = {
  add: { title: "Bulk add products", subtitle: "Fill in a template in Excel, then upload it" },
  edit: { title: "Bulk edit products", subtitle: "Download your products, change them in Excel, upload them back" },
};

export default async function ProductSheetPage({ params }: { params: Promise<{ mode: string }> }) {
  await requireRole("admin");
  const { mode } = await params;
  if (mode !== "add" && mode !== "edit") notFound();

  return (
    <>
      <PageHeader {...TITLES[mode]} />
      <ProductSheetWizard mode={mode} />
    </>
  );
}
