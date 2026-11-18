"use client";

import { LINES_SHEET } from "@/lib/product-sheet-costing";
import { rateRowsForSheet } from "../../cost-calculator/master/excel/actions";
import { runImport } from "../import/actions";
import { SheetWizard } from "../import/sheet-wizard";
import { productsForSheet } from "./actions";

const stamp = () => new Date().toISOString().slice(0, 10);
const NEW_TIP = "A new product: fill any blank row on Product Master. Its code's last part is its colour (AC/0006/NT is Natural); each colour is its own row.";
const LINES_TIPS = [
  "Cost Lines tab: one row per costing line, as in the calculator. Pick the Type, Category, Subcategory and Variety from the Rates tab; Unit, Rate and Wastage fill in.",
  "Cost Price adds up the product's lines, plus each stage's overhead % (Product Master tab).",
];

export function ProductSheetWizard({ mode }: { mode: "add" | "edit" }) {
  const add = mode === "add";
  return (
    <SheetWizard
      tips={
        add
          ? ["One row per product. Codes that already exist are skipped.", NEW_TIP, "Category must match one in Settings. Active is Yes or No.", ...LINES_TIPS, "Selling Price = Cost ÷ (1 − Markup %), rounded up to ₹10. You can type over it."]
          : ["Change any cell, or add new products.", NEW_TIP, "Don't change existing Product Codes; that's how rows are matched.", ...LINES_TIPS, "A product's Cost Lines replace its costing; one with no Cost Lines keeps it.", "Selling Price = Cost ÷ (1 − Markup %), rounded up to ₹10. You can type over it."]
      }
      downloadLabel={add ? "Download template" : "Download products"}
      fileName={add ? "new-products-template.xlsx" : `product-master-${stamp()}.xlsx`}
      saveLabel={add ? "Add products" : "Save changes"}
      saveText={({ products = 0 }) => `${add ? "Add" : "Save"} ${products} product${products === 1 ? "" : "s"}`}
      doneHref="/products"
      doneLabel="View products"
      build={async () => {
        const [{ buildProductSheet }, products, rates] = await Promise.all([
          import("@/lib/product-sheet"),
          add ? [] : productsForSheet(),
          rateRowsForSheet(),
        ]);
        return buildProductSheet(products, rates);
      }}
      check={async (grid, apply, tabs) => {
        const report = await runImport({ products: grid }, apply, mode, tabs[LINES_SHEET]);
        if (!report.applied) return report;
        const added = report.added?.length ?? 0;
        const updated = (report.counts?.products ?? 0) - added;
        const parts = [
          added && `${added} new product${added === 1 ? "" : "s"} added`,
          updated && `${updated} existing product${updated === 1 ? "" : "s"} updated`,
        ].filter(Boolean);
        return { ...report, message: `Saved: ${parts.join(", ") || "nothing to change"}.` };
      }}
    />
  );
}
