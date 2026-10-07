import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { CalculatorScreen } from "../../cost-calculator/calculator-screen";

type Search = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

export default async function NewProductPage({ searchParams }: { searchParams: Search }) {
  await requireRole("admin");
  const params = await searchParams;

  return (
    <>
      <PageHeader
        title="New product"
        subtitle="Stage-by-stage costing engine: Material, Hardware, Finishing & Machine per-minute costs"
      >
        <Link href="/products" className="btn-secondary">
          Back to products
        </Link>
        <Link href="/cost-calculator/master" className="btn-secondary">
          Rates & Hierarchy Master
        </Link>
      </PageHeader>
      <CalculatorScreen saved={one(params.saved) || undefined} t={one(params.t)} />
    </>
  );
}
