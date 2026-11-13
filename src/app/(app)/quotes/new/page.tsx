import { requireUser } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { QuoteBuilder } from "../quote-builder";
import { loadQuoteOptions } from "../data";
import { canManage } from "@/lib/supabase/server";

type Search = Promise<Record<string, string | string[] | undefined>>;
const one = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v) ?? "";

export default async function NewQuotePage({ searchParams }: { searchParams: Search }) {
  const params = await searchParams;
  const { supabase, profile } = await requireUser();
  const { hampers, products, clients, packagingCategories, settings } =
    await loadQuoteOptions(supabase);

  return (
    <>
      <PageHeader
        title="New quote"
        subtitle="The document number is assigned when you save"
      />
      <QuoteBuilder
        hampers={hampers}
        products={products}
        clients={clients}
        packagingCategories={packagingCategories}
        settings={settings}
        canEdit
        canChangeStatus={canManage(profile.role)}
        initialHamperCode={one(params.hamper) || undefined}
      />
    </>
  );
}
