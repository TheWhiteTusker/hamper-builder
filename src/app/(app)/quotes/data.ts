import type { SupabaseClient } from "@supabase/supabase-js";
import { loadSettings } from "@/lib/settings";
import type { Category, Client } from "@/lib/types";
import type { ContentItem } from "@/components/hamper-contents";
import type { HamperOption, ProductOption } from "./quote-builder";

type HamperItemRow = ContentItem & { hamper_id: string };

/**
 * Hampers offered by the quote builder's picker, plus the pick-lists and each
 * hamper's contents - the builder shows the same contents block the printed
 * document will.
 *
 * ponytail: every hamper's items in one query. Fine for a catalogue of a few
 * hundred hampers; fetch per selected hamper if it ever gets heavy.
 */
export async function loadQuoteOptions(supabase: SupabaseClient) {
  const [
    { data: hampers },
    { data: items },
    { data: categories },
    settings,
    { data: products },
    { data: clients },
  ] = await Promise.all([
    supabase
      .from("hamper_summary")
      .select("id, code, name, final_catalogue_sp")
      .order("code")
      .overrideTypes<Omit<HamperOption, "items">[], { merge: false }>(),
    supabase
      .from("hamper_items")
      .select("id, hamper_id, product_name, category_name, qty")
      .order("line_no")
      .overrideTypes<HamperItemRow[], { merge: false }>(),
    supabase.from("categories").select("*").overrideTypes<Category[], { merge: false }>(),
    loadSettings(supabase),
    // Products can go on a quote on their own, for clients who want just one item.
    supabase
      .from("products")
      .select("id, code, name, default_sp")
      .eq("is_active", true)
      .is("deleted_at", null)
      .order("code")
      .overrideTypes<ProductOption[], { merge: false }>(),
    supabase.from("clients").select("*").order("name").overrideTypes<Client[], { merge: false }>(),
  ]);

  const byHamper = new Map<string, ContentItem[]>();
  for (const item of items ?? []) {
    const list = byHamper.get(item.hamper_id) ?? [];
    list.push(item);
    byHamper.set(item.hamper_id, list);
  }

  return {
    hampers: (hampers ?? []).map((h) => ({ ...h, items: byHamper.get(h.id) ?? [] })),
    products: products ?? [],
    clients: clients ?? [],
    // Packaging is whatever Settings marks as not counting toward No. of Items.
    packagingCategories: (categories ?? []).filter((c) => !c.counts_as_item).map((c) => c.name),
    settings,
  };
}
