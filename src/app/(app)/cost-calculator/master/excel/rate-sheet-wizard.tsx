"use client";

import { SheetWizard } from "../../../admin/import/sheet-wizard";
import { importRateSheet, rateRowsForSheet } from "./actions";

const stamp = () => new Date().toISOString().slice(0, 10);

export function RateSheetWizard({ mode }: { mode: "add" | "edit" }) {
  const add = mode === "add";
  return (
    <SheetWizard
      intro={
        add
          ? "A blank sheet: one row per new variety, with its Type (e.g. Material, Hardware), Category and Subcategory. New categories and subcategories are created as needed; varieties that already exist are skipped."
          : "The whole master, one row per variety. Change rates, units, wastage or notes, or add rows at the bottom. Rows are matched on Type › Category › Subcategory › Variety, so rename things in the master screen, not here. A blank cell keeps its current value. Changing a rate or unit reprices the products that use it."
      }
      downloadLabel={add ? "Download blank sheet" : "Download the master"}
      fileName={add ? "new-rates-template.xlsx" : `rates-and-hierarchy-${stamp()}.xlsx`}
      saveLabel={add ? "Add to the master" : "Save changes"}
      doneHref="/cost-calculator/master"
      doneLabel="Back to the master"
      build={async () => {
        const { buildRateSheet, rateRows } = await import("@/lib/rate-sheet");
        return buildRateSheet(add ? [] : rateRows(await rateRowsForSheet()));
      }}
      check={(grid, apply) => importRateSheet(grid, apply, add)}
    />
  );
}
