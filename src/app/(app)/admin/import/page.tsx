import { requireRole } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { ImportWizard } from "./import-wizard";

export default async function ImportPage() {
  await requireRole("admin");

  return (
    <>
      <PageHeader
        title="Import old workbook"
        subtitle="Brings across products, saved hampers and past quotations, keeping their existing codes"
      />
      <ImportWizard />
    </>
  );
}
