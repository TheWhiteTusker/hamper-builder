import { requireUser, canManage, isAdmin } from "@/lib/supabase/server";
import { loadBinItems } from "./data";
import { BinView } from "./bin-view";

export const metadata = {
  title: "Recycle Bin — Lattice Lane",
  description: "View and restore deleted hampers, products, and photos within 30 days.",
};

export default async function BinPage() {
  const { supabase, profile } = await requireUser();
  const items = await loadBinItems(supabase);

  return (
    <BinView
      initialItems={items}
      canManage={canManage(profile.role)}
      isAdmin={isAdmin(profile.role)}
    />
  );
}
