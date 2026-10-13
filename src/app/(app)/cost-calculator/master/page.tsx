import { requireUser } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { resolveColors } from "@/lib/product-code";
import { loadCostRows } from "../load";
import { CostMasterView } from "./master-view";

export default async function CostMasterPage() {
  const { supabase } = await requireUser();
  const [rows, settings] = await Promise.all([loadCostRows(supabase), loadSettings(supabase)]);
  return (
    <CostMasterView
      rows={rows}
      productColors={resolveColors(settings.product_colors, settings.color_hex)}
    />
  );
}
