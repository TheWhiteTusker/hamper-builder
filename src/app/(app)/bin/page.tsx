import Link from "@/components/link";
import { requireUser, canManage, isAdmin } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";
import { loadBinItems } from "./data";
import { BinView } from "./bin-view";

export const metadata = {
  title: "Recycle Bin — Lattice Lane",
  description: "View and restore deleted hampers, products, and photos within 30 days.",
};

export default async function BinPage() {
  const { supabase, profile } = await requireUser();
  const { items, counts } = await loadBinItems(supabase);

  return (
    <>
      <PageHeader
        title="Recycle Bin"
        subtitle={`${counts.all} item${counts.all === 1 ? "" : "s"} in bin`}
      >
        <Link href="/settings" className="btn-secondary">
          Settings
        </Link>
      </PageHeader>
      <BinView
        initialItems={items}
        counts={counts}
        canManage={canManage(profile.role)}
        isAdmin={isAdmin(profile.role)}
      />
    </>
  );
}
