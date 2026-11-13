"use client";

import { runImport } from "../import/actions";
import { SheetWizard } from "../import/sheet-wizard";
import { productsForSheet } from "./actions";

const stamp = () => new Date().toISOString().slice(0, 10);

export function ProductSheetWizard({ mode }: { mode: "add" | "edit" }) {
  const add = mode === "add";
  return (
    <SheetWizard
      tips={
        add
          ? ["One row per product. Codes that already exist are skipped.", "Category must match one in Settings. Active is Yes or No.", "Selling Price = Cost ÷ (1 − Markup %), rounded up to ₹10. You can type over it."]
          : ["Change any cell, or add new rows at the bottom.", "Don't change existing Product Codes; that's how rows are matched.", "Selling Price = Cost ÷ (1 − Markup %), rounded up to ₹10. You can type over it."]
      }
      downloadLabel={add ? "Download template" : "Download products"}
      fileName={add ? "new-products-template.xlsx" : `product-master-${stamp()}.xlsx`}
      saveLabel={add ? "Add products" : "Save changes"}
      saveText={({ products = 0 }) => `${add ? "Add" : "Save"} ${products} product${products === 1 ? "" : "s"}`}
      doneHref="/products"
      doneLabel="View products"
      build={async () => {
        const { buildProductSheet } = await import("@/lib/product-sheet");
        return buildProductSheet(add ? [] : await productsForSheet());
      }}
      check={async (grid, apply) => {
        const report = await runImport({ products: grid }, apply, mode);
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
