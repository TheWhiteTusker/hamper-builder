export type PickListKey =
  | "hamper_statuses"
  | "quote_statuses"
  | "quote_structures"
  | "collections"
  | "sources"
  | "validity_options"
  | "detail_modes"
  | "packaging_treatments"
  | "product_colors";

export type PickListConfig = {
  key: PickListKey;
  label: string;
  hint?: string;
};

// Most-used first: the first one is open when the tab loads.
export const PICK_LIST_CONFIGS: PickListConfig[] = [
  { key: "collections", label: "Occasions / collections", hint: "Used to group hampers, e.g. Festive, Corporate, Wedding." },
  { key: "sources", label: "Vendors / sources", hint: "Where a product comes from, e.g. In-house, Outsourced." },
  { key: "product_colors", label: "Colours / finishes", hint: "Colour options offered for products." },
  { key: "hamper_statuses", label: "Hamper statuses", hint: "e.g. Draft, Approved, Active, Discontinued." },
  { key: "quote_statuses", label: "Quote statuses", hint: "Only managers can move a quote between these." },
  { key: "validity_options", label: "Quote validity", hint: "How long a quote is valid, e.g. 7 Days, 15 Days." },
  { key: "packaging_treatments", label: "Packaging treatments", hint: 'A name containing "absorb" folds packaging cost into the hamper.' },
  { key: "detail_modes", label: "Contents shown on quotes", hint: 'A name containing "hide" hides contents; "summary" prints a count.' },
  { key: "quote_structures", label: "Quote structures", hint: '"Combined Order" has one total; any other is priced per option.' },
];
