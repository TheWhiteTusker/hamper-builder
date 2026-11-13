"use client";

import { SheetWizard } from "../../../admin/import/sheet-wizard";
import { importRateSheet, rateRowsForSheet } from "./actions";

const stamp = () => new Date().toISOString().slice(0, 10);

export function RateSheetWizard({ mode }: { mode: "add" | "edit" }) {
  const add = mode === "add";
  return (
    <SheetWizard
      tips={
        add
          ? ["One row per variety: Type (e.g. Material), Category, Subcategory, Variety.", "New categories and subcategories are created for you.", "Varieties that already exist are skipped."]
          : ["Change rates, units, wastage or notes, or add rows at the bottom.", "Rename things in the master screen, not here. A blank cell keeps its value.", "A new rate or unit reprices the products that use it."]
      }
      downloadLabel={add ? "Download template" : "Download rates"}
      fileName={add ? "new-rates-template.xlsx" : `rates-and-hierarchy-${stamp()}.xlsx`}
      saveLabel={add ? "Add rates" : "Save changes"}
      saveText={({ newVarieties = 0, updatedVarieties = 0 }) => {
        const n = newVarieties + updatedVarieties;
        return add ? `Add ${n} variet${n === 1 ? "y" : "ies"}` : `Save ${n} change${n === 1 ? "" : "s"}`;
      }}
      doneHref="/cost-calculator/master"
      doneLabel="Back to rates"
      build={async () => {
        const { buildRateSheet, rateRows } = await import("@/lib/rate-sheet");
        return buildRateSheet(add ? [] : rateRows(await rateRowsForSheet()));
      }}
      check={(grid, apply) => importRateSheet(grid, apply, add)}
    />
  );
}
