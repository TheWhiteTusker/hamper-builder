"use server";

import { z } from "zod";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";
import { type ActionState, describeError } from "@/lib/forms";
import { loadSettings } from "@/lib/settings";
import { COMBINED_ORDER, optionKey } from "@/lib/pricing";

const nullableText = z.string().trim().nullable().optional();
const nullableDate = z
  .string()
  .trim()
  .nullable()
  .optional()
  .transform((v) => (v ? v : null));

const LineSchema = z.object({
  option_label: nullableText,
  hamper_id: z.string().uuid().nullable().optional(),
  hamper_code: nullableText,
  hamper_name: nullableText,
  qty: z.number(),
  catalogue_price: z.number(),
  discount_pct: z.number(),
  detail_mode: nullableText,
  packaging_treatment: nullableText,
});

const QuoteSchema = z.object({
  id: z.string().uuid().nullable().optional(),
  doc_type: z.enum(["quotation", "proforma_invoice"]),
  doc_date: z.string().trim().min(1),
  client_name: z.string().trim().min(1, "Client or company name is required"),
  contact_person: nullableText,
  phone: nullableText,
  email: nullableText,
  billing_address: nullableText,
  gstin: nullableText,
  occasion: nullableText,
  quote_structure: z.string().trim().min(1),
  validity: nullableText,
  status: z.string().trim().min(1),
  order_discount: z.number(),
  adj1: z.number(),
  adj2: z.number(),
  gst_rate: z.number(),
  notes: nullableText,
  terms: nullableText,
  follow_up_date: nullableDate,
  linked_doc_no: nullableText,
  lines: z.array(LineSchema).min(1, "Add at least one hamper or product to the quotation"),
});

export async function saveQuote(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  let payload: unknown;

  try {
    payload = JSON.parse(String(formData.get("payload") ?? "{}"));
  } catch {
    return { error: "Could not read the quotation. Please try again." };
  }

  const parsed = QuoteSchema.safeParse(payload);
  if (!parsed.success) return { error: parsed.error.issues[0].message };

  const supabase = await createClient();
  const { data: docNo, error } = await supabase.rpc("save_quote", { p: parsed.data });

  if (error) return { error: describeError(error) };

  revalidatePath("/quotes");
  redirect(`/quotes/${encodeURIComponent(String(docNo))}?saved=1`);
}

export async function deleteQuote(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const id = String(formData.get("id") ?? "");
  if (!id) return { error: "Missing quotation." };

  const supabase = await createClient();
  const { error } = await supabase.from("quotes").delete().eq("id", id);

  if (error) return { error: describeError(error) };

  revalidatePath("/quotes");
  redirect("/quotes?deleted=1");
}

/**
 * Turns an accepted quotation into a proforma invoice: a new document with its
 * own LLPI- number, linked back to the quotation it came from. The Apps Script
 * had a "Linked Document No." column but nothing that ever filled it in.
 *
 * From an option-based quotation only the chosen option is carried across, and
 * the invoice is a combined order: those are the confirmed items.
 */
export async function convertToProforma(
  _prev: ActionState,
  formData: FormData,
): Promise<ActionState> {
  const docNo = String(formData.get("doc_no") ?? "");
  const option = String(formData.get("option") ?? "");
  if (!docNo) return { error: "Missing quotation." };

  const supabase = await createClient();

  const { data: quote, error: readError } = await supabase
    .from("quotes")
    .select("*, quote_items(*)")
    .eq("doc_no", docNo)
    .single();

  if (readError || !quote) return { error: describeError(readError ?? "Not found") };

  const { pi_terms } = await loadSettings(supabase);

  const { quote_items: allLines, id: _id, doc_no: _docNo, created_at, updated_at, ...rest } = quote;
  const isOptions = rest.quote_structure !== COMBINED_ORDER;
  if (isOptions && !option) return { error: "Choose the option the client picked." };
  const lines = isOptions
    ? (allLines ?? []).filter((l: { option_label: string | null }) => optionKey(l.option_label) === option)
    : allLines;
  if (!lines?.length) return { error: isOptions ? `${option} has no items to invoice.` : "This quotation has no items." };
  void _id;
  void _docNo;
  void created_at;
  void updated_at;

  const { data: newNo, error } = await supabase.rpc("save_quote", {
    p: {
      ...rest,
      doc_type: "proforma_invoice",
      quote_structure: COMBINED_ORDER,
      // An option is priced without order discount or charges; keep the invoice equal to it.
      ...(isOptions && { order_discount: 0, adj1: 0, adj2: 0 }),
      status: "Draft",
      linked_doc_no: docNo,
      // A PI carries payment terms (advance, delivery), not quotation terms.
      terms: pi_terms || rest.terms,
      lines: (lines ?? []).map(
        (l: {
          option_label: string | null;
          hamper_id: string | null;
          hamper_code: string | null;
          hamper_name: string | null;
          qty: number;
          catalogue_price: number;
          discount_pct: number;
          detail_mode: string | null;
          packaging_treatment: string | null;
        }) => ({
          option_label: null,
          hamper_id: l.hamper_id,
          hamper_code: l.hamper_code,
          hamper_name: l.hamper_name,
          qty: l.qty,
          catalogue_price: l.catalogue_price,
          discount_pct: l.discount_pct,
          detail_mode: l.detail_mode,
          packaging_treatment: l.packaging_treatment,
        }),
      ),
    },
  });

  if (error) return { error: describeError(error) };

  revalidatePath("/quotes");
  redirect(`/quotes/${encodeURIComponent(String(newNo))}?converted=${encodeURIComponent(docNo)}`);
}
