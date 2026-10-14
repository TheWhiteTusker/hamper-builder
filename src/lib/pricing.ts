/**
 * Every number in the app comes from here.
 *
 * These functions are the client-side mirror of the `hamper_summary` and
 * `quote_summary` SQL views, used to show live totals while someone is typing.
 * The database is still the authority once a record is saved - if the two ever
 * disagree, the view is right and this file is the bug.
 *
 * The formulas are lifted from the Apps Script, chiefly refreshHamperPrices()
 * which is the only place the spreadsheet spelled them out in code rather than
 * hiding them in cell formulas.
 */

import { num, round2, roundUpToNext10 } from "./numbers.ts";

export * from "./numbers.ts";
export * from "./money-format.ts";

// ---------------------------------------------------------------
// HAMPERS
// ---------------------------------------------------------------

export type HamperLineInput = {
  qty: number;
  unitCp: number;
  unitSp: number;
  /** categoryCountsAsItem(): unknown categories count, so this defaults true. */
  countsAsItem?: boolean;
};

export type HamperPricing = {
  totalCp: number;
  baseSp: number;
  numberOfItems: number;
  spAfterDiscount: number;
  /** null until a Target SP is set. */
  variance: number | null;
  /** null until a Final Catalogue SP is approved. */
  grossProfit: number | null;
  finalMargin: number | null;
};

export function priceHamper(
  lines: HamperLineInput[],
  opts: {
    discountPct?: number;
    targetSp?: number | null;
    finalCatalogueSp?: number | null;
  } = {},
): HamperPricing {
  const discountPct = num(opts.discountPct);

  let totalCp = 0;
  let baseSp = 0;
  let numberOfItems = 0;

  for (const line of lines) {
    const qty = num(line.qty);
    totalCp += qty * num(line.unitCp);
    baseSp += qty * num(line.unitSp);
    // Inside packaging adds cost but is not one of the "items" advertised.
    if (line.countsAsItem !== false) numberOfItems += qty;
  }

  const spAfterDiscount = baseSp * (1 - discountPct);

  const targetSp = opts.targetSp ?? null;
  const finalSp = opts.finalCatalogueSp ?? null;

  // Gross profit is measured against the *approved* catalogue price, never the
  // computed one. refreshHamperPrices() preserves that override on purpose.
  const grossProfit = finalSp === null ? null : finalSp - totalCp;

  return {
    totalCp,
    baseSp,
    numberOfItems,
    spAfterDiscount,
    variance: targetSp === null ? null : spAfterDiscount - targetSp,
    grossProfit,
    finalMargin:
      finalSp === null || finalSp === 0 || grossProfit === null
        ? null
        : grossProfit / finalSp,
  };
}

/**
 * A hamper line's margin (a whole percent, as typed) and unit price follow
 * each other the way the calculator's markup and SP do: price = cost ÷
 * (1 − margin), rounded up to the next 10. null when there is no cost to
 * price from, or at 100%+, which has no finite price.
 */
export const spAtMargin = (cp: number, marginPct: number): number | null => {
  const divisor = 1 - marginPct / 100;
  return cp > 0 && divisor > 0 ? roundUpToNext10(cp / divisor) : null;
};

/** Margin as a whole percent, (price − cost) ÷ price; null without a price. */
export const marginPctOf = (cp: number, sp: number): number | null =>
  sp > 0 ? round2(((sp - cp) / sp) * 100) : null;

// ---------------------------------------------------------------
// QUOTES
// ---------------------------------------------------------------

export type QuoteLineInput = {
  qty: number;
  cataloguePrice: number;
  discountPct?: number;
};

/** Sheet columns G and H, now generated columns on quote_items. */
export function priceQuoteLine(line: QuoteLineInput) {
  const finalRate = num(line.cataloguePrice) * (1 - num(line.discountPct));
  return { finalRate, amount: num(line.qty) * finalRate };
}

export const COMBINED_ORDER = "Combined Order";

export type QuotePricing = {
  subtotal: number;
  totalQty: number;
  /** null for option-style quotes: the client picks one, so there is no total. */
  taxableValue: number | null;
  gstAmount: number | null;
  grandTotal: number | null;
};

export function priceQuote(
  lines: QuoteLineInput[],
  opts: {
    quoteStructure?: string;
    orderDiscount?: number;
    adj1?: number;
    adj2?: number;
    gstRate?: number;
  } = {},
): QuotePricing {
  let subtotal = 0;
  let totalQty = 0;

  for (const line of lines) {
    subtotal += priceQuoteLine(line).amount;
    totalQty += num(line.qty);
  }

  // Mirrors =IF(B7="Combined Order", ...) on H22/H26/H28/H29.
  if (opts.quoteStructure !== COMBINED_ORDER) {
    return { subtotal, totalQty, taxableValue: null, gstAmount: null, grandTotal: null };
  }

  const taxableValue =
    subtotal - num(opts.orderDiscount) + num(opts.adj1) + num(opts.adj2);
  const gstAmount = taxableValue * num(opts.gstRate);

  return {
    subtotal,
    totalQty,
    taxableValue,
    gstAmount,
    grandTotal: taxableValue + gstAmount,
  };
}
