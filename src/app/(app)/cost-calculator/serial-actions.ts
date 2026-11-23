"use server";

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { getNextSerialForCategory } from "@/lib/product-code";

/** The next free serial for a category code, for a new product's code. */
export async function getNextSerialAction(
  categoryCode: string,
): Promise<{ serial: string; error?: string }> {
  try {
    const supabase = await createClient();
    const { data: products, error } = await supabase.from("products").select("code");
    if (error) return { serial: "0001", error: describeError(error) };
    const codes = (products ?? []).map((p: { code: string }) => p.code);
    return { serial: getNextSerialForCategory(categoryCode, codes) };
  } catch (err: unknown) {
    return { serial: "0001", error: describeError(err) };
  }
}
