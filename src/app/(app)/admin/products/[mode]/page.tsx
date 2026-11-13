import { notFound } from "next/navigation";
import { requireRole } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { ProductSheetWizard } from "../product-sheet-wizard";

const TITLES = {
  add: { title: "Add new products", subtitle: "Download a blank Excel sheet, fill in new products, upload it back" },
  edit: { title: "Edit & add products", subtitle: "Download every product as Excel, change or add rows, upload it back" },
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
