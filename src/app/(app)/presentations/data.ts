import type { SupabaseClient } from "@supabase/supabase-js";
import type { PickerData } from "./item-picker";

/** Hampers and active products for the slide picker. */
export async function loadPickerData(supabase: SupabaseClient): Promise<PickerData> {
  const [{ data: hampers }, { data: products }] = await Promise.all([
    supabase
      .from("hamper_summary")
      .select("id, code, name, final_catalogue_sp, image_url")
      .order("code")
      .overrideTypes<{ id: string; code: string; name: string; final_catalogue_sp: number | null; image_url: string | null }[], { merge: false }>(),
    supabase
      .from("products")
      .select("id, code, name, default_sp, image_url")
      .eq("is_active", true)
      .is("deleted_at", null)
      .order("name")
      .overrideTypes<{ id: string; code: string; name: string; default_sp: number | null; image_url: string | null }[], { merge: false }>(),
  ]);

  return {
    hampers: (hampers ?? []).map((h) => ({
      type: "hamper",
      id: h.id,
      code: h.code,
      name: h.name,
      price: h.final_catalogue_sp == null ? null : Number(h.final_catalogue_sp),
      image_url: h.image_url,
    })),
    products: (products ?? []).map((p) => ({
      type: "product",
      id: p.id,
      code: p.code,
      name: p.name,
      price: p.default_sp == null ? null : Number(p.default_sp),
      image_url: p.image_url,
    })),
  };
}
