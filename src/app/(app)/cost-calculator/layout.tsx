import { PageHeader } from "@/components/ui";
import { isAdmin, requireUser } from "@/lib/supabase/server";
import { CostTabs } from "./tabs";

export default async function CostCalculatorLayout({ children }: { children: React.ReactNode }) {
  const { profile } = await requireUser();
  return (
    <>
      <PageHeader
        title="Product Cost Calculator"
        subtitle="Work out what a product costs: materials, hardware, finishing and machine time"
      >
        <CostTabs admin={isAdmin(profile.role)} />
      </PageHeader>
      {children}
    </>
  );
}
