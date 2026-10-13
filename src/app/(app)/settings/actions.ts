"use server";

import { z } from "zod";
import { createClient } from "@/lib/supabase/server";
import { type ActionState, describeError, percent } from "@/lib/forms";

async function saveSetting(key: string, value: unknown): Promise<ActionState> {
  const supabase = await createClient();
  const { error } = await supabase.from("app_settings").upsert({ key, value }, { onConflict: "key" });

  if (error) return { error: describeError(error) };

  // No revalidatePath: it re-renders the whole settings page. The forms keep
  // what was typed, and other pages refresh on the next navigation.
  return { ok: true };
}

// ---------------------------------------------------------------
// COMPANY + TAX
// ---------------------------------------------------------------

const CompanySchema = z.object({
  name: z.string().trim().min(1, "Company name is required"),
  legal_name: z.string().trim(),
  address: z.string().trim(),
  gstin: z.string().trim(),
  phone: z.string().trim(),
  email: z.string().trim(),
  website: z.string().trim(),
  bank_account_name: z.string().trim(),
  bank_name: z.string().trim(),
  bank_account_no: z.string().trim(),
  bank_ifsc: z.string().trim().toUpperCase(),
});

export async function saveCompany(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const parsed = CompanySchema.safeParse(Object.fromEntries(formData));
  if (!parsed.success) return { error: parsed.error.issues[0].message };

  const company = await saveSetting("company", parsed.data);
  if (company.error) return company;

  const gst = percent.safeParse(String(formData.get("gst_rate") ?? ""));
  if (gst.success) {
    const rate = await saveSetting("gst_rate", gst.data);
    if (rate.error) return rate;
  }

  return { ok: true };
}

// ---------------------------------------------------------------
// PICK-LISTS
// ---------------------------------------------------------------

/** Add a single item to a pick-list in app_settings. */
export async function addPickListItem(
  key: string,
  rawItem: string,
): Promise<ActionState & { values?: string[] }> {
  const trimmed = rawItem.trim();
  if (!key) return { error: "Missing list key." };
  if (!trimmed) return { error: "Item name cannot be empty." };

  const supabase = await createClient();
  const { data, error } = await supabase
    .from("app_settings")
    .select("value")
    .eq("key", key)
    .maybeSingle();

  if (error) return { error: describeError(error) };

  const current: string[] = Array.isArray(data?.value) ? (data.value as string[]) : [];
  if (current.some((v) => v.toLowerCase() === trimmed.toLowerCase())) {
    return { error: `"${trimmed}" already exists in this pick-list.` };
  }

  const updated = [...current, trimmed];
  const res = await saveSetting(key, updated);
  if (res.error) return res;

  return { ok: true, values: updated };
}

/** Remove a single item from a pick-list in app_settings. */
export async function removePickListItem(
  key: string,
  item: string,
): Promise<ActionState & { values?: string[] }> {
  if (!key) return { error: "Missing list key." };

  const supabase = await createClient();
  const { data, error } = await supabase
    .from("app_settings")
    .select("value")
    .eq("key", key)
    .maybeSingle();

  if (error) return { error: describeError(error) };

  const current: string[] = Array.isArray(data?.value) ? (data.value as string[]) : [];
  const updated = current.filter((v) => v !== item);
  const res = await saveSetting(key, updated);
  if (res.error) return res;

  return { ok: true, values: updated };
}

export async function saveDefault(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const key = String(formData.get("key") ?? "");
  if (!key) return { error: "Missing setting." };

  return saveSetting(key, String(formData.get("value") ?? ""));
}
