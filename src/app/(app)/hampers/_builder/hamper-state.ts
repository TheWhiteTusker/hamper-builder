import { marginPctOf, num, round2, spAtMargin } from "@/lib/pricing";
import type { Hamper, HamperItem } from "@/lib/types";

/** The slice of Product Master the builder needs in the browser. */
export type CatalogProduct = {
  id: string;
  code: string;
  name: string;
  category_id: string | null;
  category_name: string | null;
  counts_as_item: boolean;
  source: string | null;
  cost_price: number;
  target_margin: number;
  default_sp: number;
  colors?: string[];
  markup_pct?: number;
  image_url?: string | null;
};

/** A hamper line as typed: numbers stay strings until saved; margin is a whole percent. */
export type Line = {
  key: string;
  product_id: string | null;
  product_code: string;
  product_name: string;
  category_name: string;
  source: string;
  qty: string;
  unit_cp: string;
  unit_sp: string;
  target_margin: string;
  /** Margin or price typed by hand: saving makes it the product's price too. */
  repriced: boolean;
};

/** Hamper details as typed; discount is a whole percent. */
export type HamperFields = {
  name: string;
  collection: string;
  status: string;
  notes: string;
  targetSp: string;
  discountPct: string;
  finalSp: string;
};

export const initialFields = (hamper?: Hamper): HamperFields => ({
  name: hamper?.name ?? "",
  collection: hamper?.collection ?? "",
  status: hamper?.status ?? "Draft",
  notes: hamper?.notes ?? "",
  targetSp: hamper?.target_sp != null ? String(hamper.target_sp) : "",
  discountPct: hamper ? String(round2(hamper.discount_pct * 100)) : "0",
  finalSp: hamper?.final_catalogue_sp != null ? String(hamper.final_catalogue_sp) : "",
});

export const blankLine = (): Line => ({
  key: crypto.randomUUID(),
  product_id: null,
  product_code: "",
  product_name: "",
  category_name: "",
  source: "",
  qty: "1",
  unit_cp: "",
  unit_sp: "",
  target_margin: "",
  repriced: false,
});

export const toLine = (item: HamperItem): Line => ({
  key: crypto.randomUUID(),
  product_id: item.product_id,
  product_code: item.product_code ?? "",
  product_name: item.product_name,
  category_name: item.category_name ?? "",
  source: item.source ?? "",
  qty: String(item.qty),
  unit_cp: String(item.unit_cp),
  unit_sp: String(item.unit_sp),
  target_margin: String(round2(item.target_margin * 100)),
  repriced: false,
});

/** Same job as populateProductRow() in the Apps Script: a product fills its line. */
export const productPatch = (p: CatalogProduct | undefined): Partial<Line> =>
  p
    ? {
        product_id: p.id,
        product_code: p.code,
        product_name: p.name,
        category_name: p.category_name ?? "",
        source: p.source ?? "",
        unit_cp: String(p.cost_price),
        unit_sp: String(p.default_sp),
        target_margin: String(round2(p.target_margin * 100)),
        repriced: false,
      }
    : { product_id: null, product_code: "", product_name: "", repriced: false };

/**
 * Margin and unit price follow each other: a margin prices the line from its
 * cost, a typed price works out the margin. A new cost keeps the price and
 * moves the margin, as a cost change does in the calculator.
 */
export function priceEdit(line: Line, key: "unit_cp" | "unit_sp" | "target_margin", value: string): Partial<Line> {
  const cp = num(key === "unit_cp" ? value : line.unit_cp);
  if (key === "target_margin") {
    const sp = value === "" ? null : spAtMargin(cp, num(value));
    return { target_margin: value, repriced: true, ...(sp !== null && { unit_sp: String(sp) }) };
  }
  const margin = marginPctOf(cp, num(key === "unit_sp" ? value : line.unit_sp));
  return {
    [key]: value,
    ...(margin !== null && { target_margin: String(margin) }),
    ...(key === "unit_sp" && { repriced: true }),
  };
}

/** What a product looks like in the search box - name plus code, so typing
    either one narrows the list, and the text maps back to exactly one product. */
export const productLabel = (name: string, code: string) => (code ? `${name} — ${code}` : name);

const optionalNum = (v: string) => (v === "" ? null : num(v));

/** The pricing options priceHamper() takes. */
export const pricingOpts = (f: HamperFields) => ({
  discountPct: num(f.discountPct) / 100,
  targetSp: optionalNum(f.targetSp),
  finalCatalogueSp: optionalNum(f.finalSp),
});

/** The JSON the saveHamper action receives; blank lines are left out. */
export const hamperPayload = (f: HamperFields, lines: Line[], hamper?: Hamper) =>
  JSON.stringify({
    id: hamper?.id ?? null,
    name: f.name,
    collection: f.collection || null,
    status: f.status,
    target_sp: optionalNum(f.targetSp),
    notes: f.notes || null,
    discount_pct: num(f.discountPct) / 100,
    final_catalogue_sp: optionalNum(f.finalSp),
    lines: lines
      .filter((l) => l.product_name.trim() !== "")
      .map((l) => ({
        product_id: l.product_id,
        product_code: l.product_code || null,
        product_name: l.product_name,
        category_name: l.category_name || null,
        source: l.source || null,
        qty: num(l.qty),
        unit_cp: num(l.unit_cp),
        unit_sp: num(l.unit_sp),
        target_margin: num(l.target_margin) / 100,
        reprice: l.repriced,
      })),
  });
