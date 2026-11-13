/** What buildImport() hands back: rows ready to insert, plus warnings. */

export type ImportResult = {
  categories: { name: string; counts_as_item: boolean; sort_order: number }[];
  defaultDetailMode: string | null;
  products: {
    code: string;
    category_name: string | null;
    name: string;
    source: string | null;
    cost_price: number;
    target_margin: number;
    default_sp: number;
    is_active: boolean;
    /** Whole percent; only the Product Master Excel sheet sets it. */
    markup_pct?: number;
  }[];
  hampers: {
    code: string;
    name: string;
    collection: string | null;
    status: string;
    discount_pct: number;
    target_sp: number | null;
    final_catalogue_sp: number | null;
    notes: string | null;
  }[];
  hamperItems: {
    hamper_code: string;
    line_no: number;
    product_code: string | null;
    product_name: string;
    category_name: string | null;
    source: string | null;
    qty: number;
    unit_cp: number;
    unit_sp: number;
    target_margin: number;
  }[];
  quotes: {
    doc_no: string;
    doc_type: "quotation" | "proforma_invoice";
    doc_date: string | null;
    client_name: string;
    contact_person: string | null;
    phone: string | null;
    email: string | null;
    occasion: string | null;
    quote_structure: string;
    status: string;
    validity: string | null;
    follow_up_date: string | null;
    notes: string | null;
    linked_doc_no: string | null;
    gst_rate: number;
    order_discount: number;
    adj1: number;
    adj2: number;
  }[];
  quoteItems: {
    doc_no: string;
    line_no: number;
    option_label: string | null;
    hamper_code: string | null;
    hamper_name: string | null;
    qty: number;
    catalogue_price: number;
    discount_pct: number;
    detail_mode: string | null;
    packaging_treatment: string | null;
  }[];
  warnings: string[];
  counts: Record<string, number>;
};
