import { calculateCostSheetTotals, calculateLineCost, scaledPrice } from "./costing.ts";
import { parseProductCode } from "./product-code.ts";
import type { ProductCostLine, ProductCostSheet } from "./types";

export type MasterRate = {
  id: string;
  name: string;
  default_rate: number;
  unit: string;
  subcategory_name?: string | null;
  category_name?: string | null;
};
export type PricedProduct = { id: string; code: string; name: string; cost_price: number; default_sp: number };

/**
 * The products a cost sheet prices: its own product, plus the colour
 * siblings saved from it (LC/0001/WL -> LC/0001/BL), unless a sibling has
 * since been costed on a sheet of its own.
 */
function productsOfSheet(sheet: ProductCostSheet, products: PricedProduct[], costedAlone: Set<string>) {
  const ids = new Set<string>(sheet.product_id ? [sheet.product_id] : []);
  const { categoryCode, serial, isValid } = parseProductCode(sheet.product_code ?? "");
  if (!isValid) return [...ids];
  const prefix = `${categoryCode}/${serial}/`;
  for (const p of products) if (p.code.startsWith(prefix) && !costedAlone.has(p.id)) ids.add(p.id);
  return [...ids];
}

/**
 * Re-rates every line using one of `rates`, recomputes their sheets (with
 * overheads) and reprices those sheets' products, scaling each selling price
 * with its cost so the margin holds. Pure: returns the rows to write.
 * `lines` must hold every line of the affected sheets.
 */
export function planReprice(input: {
  rates: MasterRate[];
  lines: ProductCostLine[];
  sheets: ProductCostSheet[];
  products: PricedProduct[];
  costedAlone: Set<string>;
}) {
  const rates = new Map(input.rates.map((r) => [r.id, r]));

  const lines = input.lines.map((l) => {
    const v = l.cost_variety_id ? rates.get(l.cost_variety_id) : undefined;
    if (!v) return { line: l, changed: false };
    const rate = Number(v.default_rate);
    const calc = calculateLineCost({ ...l, rate, unit: v.unit });
    // Lines also take the master's current names, so renames show everywhere.
    const line: ProductCostLine = {
      ...l,
      ...(v.category_name ? { category_name: v.category_name } : {}),
      ...(v.subcategory_name ? { subcategory_name: v.subcategory_name, item_name: `${v.subcategory_name} ${v.name}` } : {}),
      variety_name: v.name,
      rate,
      unit: v.unit,
      calculated_area: calc.calculated_area,
      line_total: calc.line_total,
    };
    return { line, changed: true };
  });

  const changedSheets = new Set(lines.filter((l) => l.changed).map((l) => l.line.sheet_id));
  const productRows = new Map<string, { id: string; code: string; name: string; cost_price: number; default_sp: number; target_margin: number }>();
  const sheetRows = [];

  for (const sheet of input.sheets.filter((s) => changedSheets.has(s.id))) {
    const t = calculateCostSheetTotals(
      lines.filter((l) => l.line.sheet_id === sheet.id).map((l) => l.line),
      sheet.markup_pct,
      sheet.stage_overheads ?? {},
    );
    const ids = new Set(productsOfSheet(sheet, input.products, input.costedAlone));
    for (const p of input.products.filter((p) => ids.has(p.id))) {
      const sp = scaledPrice(p.cost_price, p.default_sp, t.total_cost, t.calculated_sp);
      productRows.set(p.id, {
        id: p.id,
        code: p.code,
        name: p.name,
        cost_price: t.total_cost,
        default_sp: sp,
        target_margin: sp > 0 ? (sp - t.total_cost) / sp : 0,
      });
    }
    // The sheet records the price its own product now sells at.
    sheetRows.push({
      id: sheet.id,
      product_name: sheet.product_name,
      material_total: t.material_total,
      hardware_total: t.hardware_total,
      finishing_total: t.finishing_total,
      machine_total: t.machine_total,
      total_cost: t.total_cost,
      calculated_sp: (sheet.product_id ? productRows.get(sheet.product_id)?.default_sp : undefined) ?? t.calculated_sp,
    });
  }

  return {
    lines: lines.filter((l) => l.changed).map((l) => l.line),
    sheets: sheetRows,
    products: [...productRows.values()],
  };
}
