"use client";

import { runImport } from "../import/actions";
import { SheetWizard } from "../import/sheet-wizard";
import { productsForSheet } from "./actions";

const stamp = () => new Date().toISOString().slice(0, 10);

export function ProductSheetWizard({ mode }: { mode: "add" | "edit" }) {
  const add = mode === "add";
  return (
    <SheetWizard
      intro={
        add
          ? "A blank sheet with the column headings. One row per new product; codes that already exist are skipped. Default SP = Cost Price ÷ (1 − Markup %), or type over it. Category must match an existing category; Active is Yes or No."
          : "Every product, ready to edit. Change any cell, or add new rows at the bottom. Rows are matched on Product Code, so don't change existing codes. Default SP = Cost Price ÷ (1 − Markup %), or type over it."
      }
      downloadLabel={add ? "Download blank sheet" : "Download all products"}
      fileName={add ? "new-products-template.xlsx" : `product-master-${stamp()}.xlsx`}
      saveLabel={add ? "Add these products" : "Save changes"}
      doneHref="/products"
      doneLabel="View Product Master"
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
