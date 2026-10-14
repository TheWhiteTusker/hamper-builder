import type { SupabaseClient } from "@supabase/supabase-js";
import { describeError } from "@/lib/forms";
import { round2 } from "@/lib/pricing";

/** A hamper line whose margin or price was typed by hand. */
type RepricedLine = { product_id: string; unit_sp: number; target_margin: number };

/**
 * Makes a price set on a hamper line the product's own price in Product
 * Master and on its cost sheet, so the product list and the calculator agree
 * with the hamper. The line's margin is the calculator's markup (SP = CP ÷
 * (1 − markup)). Other hampers keep their prices until Refresh prices.
 *
 * Upserts on id: a request per table however many lines changed, because a
 * Worker invocation caps its outgoing requests.
 */
export async function pushLinePrices(supabase: SupabaseClient, lines: RepricedLine[]): Promise<string | undefined> {
  // The same product twice: the later line wins.
  const byProduct = new Map(lines.map((l) => [l.product_id, l]));
  if (!byProduct.size) return;
  const ids = [...byProduct.keys()];

  const [products, sheets] = await Promise.all([
    supabase
      .from("products")
      .select("id, code, name, cost_price")
      .in("id", ids)
      .returns<{ id: string; code: string; name: string; cost_price: number }[]>(),
    supabase
      .from("product_cost_sheets")
      .select("id, product_id, product_name")
      .in("product_id", ids)
      .returns<{ id: string; product_id: string; product_name: string }[]>(),
  ]);
  const readError = products.error ?? sheets.error;
  if (readError) return describeError(readError);

  const priceOf = (id: string) => {
    const l = byProduct.get(id)!;
    return { markup_pct: round2(l.target_margin * 100), sp: l.unit_sp };
  };

  const productRows = (products.data ?? []).map((p) => {
    const { markup_pct, sp } = priceOf(p.id);
    const cp = Number(p.cost_price);
    return { id: p.id, code: p.code, name: p.name, markup_pct, default_sp: sp, target_margin: sp > 0 ? (sp - cp) / sp : 0 };
  });
  const sheetRows = (sheets.data ?? []).map((s) => {
    const { markup_pct, sp } = priceOf(s.product_id);
    return { id: s.id, product_name: s.product_name, markup_pct, calculated_sp: sp };
  });

  if (productRows.length) {
    const { error } = await supabase.from("products").upsert(productRows, { onConflict: "id" });
    if (error) return describeError(error);
  }
  if (sheetRows.length) {
    const { error } = await supabase.from("product_cost_sheets").upsert(sheetRows, { onConflict: "id" });
    if (error) return describeError(error);
  }
}
