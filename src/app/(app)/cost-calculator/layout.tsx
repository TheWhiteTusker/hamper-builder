import { PageHeader } from "@/components/ui";
import { CostTabs } from "./tabs";

export default function CostCalculatorLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <PageHeader
        title="Product Cost Calculator"
        subtitle="Work out what a product costs: materials, hardware, finishing and machine time"
      >
        <CostTabs />
      </PageHeader>
      {children}
    </>
  );
}
