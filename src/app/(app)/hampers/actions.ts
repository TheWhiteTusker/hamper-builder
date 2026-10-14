"use server";

import { z } from "zod";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createClient, requireUser } from "@/lib/supabase/server";
import { type ActionState, describeError } from "@/lib/forms";
import { pushLinePrices } from "./push-prices";

const nullableText = z.string().trim().nullable().optional();

const LineSchema = z.object({
  product_id: z.string().uuid().nullable().optional(),
  product_code: nullableText,
  product_name: z.string().trim().min(1),
  category_name: nullableText,
  source: nullableText,
  qty: z.number(),
  unit_cp: z.number(),
  unit_sp: z.number(),
  target_margin: z.number(),
  /** Margin or price typed by hand on this line. */
  reprice: z.boolean().optional(),
});

const HamperSchema = z.object({
  id: z.string().uuid().nullable().optional(),
  name: z.string().trim().min(1, "Hamper name is required"),
  collection: nullableText,
  status: z.string().trim().default("Draft"),
  target_sp: z.number().nullable().optional(),
  notes: nullableText,
  discount_pct: z.number().default(0),
  final_catalogue_sp: z.number().nullable().optional(),
  lines: z.array(LineSchema).min(1, "Add at least one product to the hamper"),
});

export async function saveHamper(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  let payload: unknown;

  try {
    payload = JSON.parse(String(formData.get("payload") ?? "{}"));
  } catch {
    return { error: "Could not read the hamper. Please try again." };
  }

  const parsed = HamperSchema.safeParse(payload);
  if (!parsed.success) return { error: parsed.error.issues[0].message };

  // The spreadsheet refused to save without a Final Catalogue SP. That makes
  // sense for a hamper going into a catalogue, but not for one still being
  // costed - so the rule applies only once it leaves Draft.
  if (parsed.data.status !== "Draft" && parsed.data.final_catalogue_sp == null) {
    return {
      error: `A Final Catalogue SP is required before a hamper can be marked ${parsed.data.status}.`,
    };
  }

  const { supabase, profile } = await requireUser();

  // Prices typed on a line become the product's price too. First, because it
  // is safe to repeat: a retry after a failure cannot create a second hamper.
  // Product Master is admin-only (RLS); a manager's prices stay on the hamper.
  const repriced = parsed.data.lines.flatMap((l) => (l.reprice && l.product_id ? [{ ...l, product_id: l.product_id }] : []));
  if (profile.role === "admin" && repriced.length) {
    const pushError = await pushLinePrices(supabase, repriced);
    if (pushError) return { error: `Could not update the product prices: ${pushError}` };
    revalidatePath("/products", "layout");
    revalidatePath("/cost-calculator", "layout");
  }

  const { data: code, error } = await supabase.rpc("save_hamper", { p: parsed.data });

  if (error) return { error: describeError(error) };

  revalidatePath("/hampers");
  // "layout" covers both the preview and its /edit page.
  revalidatePath(`/hampers/${encodeURIComponent(String(code))}`, "layout");
  // Saving is the end of the job, so it lands back on the list rather than
  // leaving you parked on the hamper you just finished.
  redirect("/hampers");
}

export async function duplicateHamper(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const code = String(formData.get("code") ?? "");
  if (!code) return { error: "Missing hamper." };

  const supabase = await createClient();
  const { data: newCode, error } = await supabase.rpc("duplicate_hamper", {
    p_code: code,
  });

  if (error) return { error: describeError(error) };

  revalidatePath("/hampers");
  redirect(`/hampers/${encodeURIComponent(String(newCode))}/edit?duplicated=${encodeURIComponent(code)}`);
}

export async function deleteHamper(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const id = String(formData.get("id") ?? "");
  if (!id) return { error: "Missing hamper." };

  const supabase = await createClient();

  // Soft delete: move to bin
  const { error } = await supabase
    .from("hampers")
    .update({ deleted_at: new Date().toISOString() })
    .eq("id", id);
  if (error) return { error: describeError(error) };

  revalidatePath("/hampers");
  revalidatePath("/bin");
  redirect("/hampers?deleted=1");
}
