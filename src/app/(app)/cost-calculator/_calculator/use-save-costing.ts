import { useState, useTransition } from "react";
import { formatMoney, num } from "@/lib/pricing";
import type { ProductCostSheet } from "@/lib/types";
import { costingSaved, saveCostSheetAndProduct } from "../actions";
import { toSavedLine, type LineState } from "./lines";
import type { Pricing } from "./use-pricing";
import type { ProductDetails } from "./use-product-details";
import type { ProductPhotos } from "./use-product-photos";

export type Feedback = { error?: string; success?: string };

/** Saves the product and its cost sheet, then uploads any queued photos. */
export function useSaveCosting({
  details,
  pricing,
  photos,
  activeLines,
  initialSheet,
}: {
  details: ProductDetails;
  pricing: Pricing;
  photos: ProductPhotos;
  activeLines: LineState[];
  initialSheet?: ProductCostSheet | null;
}) {
  const [isPending, startTransition] = useTransition();
  const [feedback, setFeedback] = useState<Feedback>({});

  function save() {
    setFeedback({});
    const code = details.code.trim();
    if (!code) return setFeedback({ error: "Product Code is required." });
    if (!details.name.trim()) return setFeedback({ error: "Product Name is required." });

    startTransition(async () => {
      const res = await saveCostSheetAndProduct({
        sheetId: initialSheet?.id ?? null,
        productId: details.selectedProductId || null,
        productCode: code,
        productName: details.name.trim(),
        categoryId: details.categoryId || null,
        source: details.source.trim() || null,
        colors: details.selectedColors,
        isActive: details.isActive,
        markupPct: num(pricing.markupPct),
        stageOverheads: pricing.stageOverheads,
        sellingPrice: pricing.effectiveSp,
        notes: details.notes.trim() || null,
        // Skip starter rows left empty
        lines: activeLines
          .filter((l) => l.category_name || l.item_name || num(l.rate) > 0)
          .map(toSavedLine),
      });
      if (res.error) return setFeedback({ error: res.error });

      const ids = Object.fromEntries((res.variants ?? []).map((v) => [v.color, v.id]));
      const photo = await photos.afterSave(ids, res.productId);
      const savedCodes = res.variants?.length
        ? res.variants.map((v) => v.code).join(", ")
        : (res.productCode ?? code);

      // All good: refresh and open that product's page in one request.
      if (!photo.failed) {
        await costingSaved(res.productCode ?? code);
        return;
      }
      await costingSaved();

      // A photo failed: stay put so the error is visible and can be retried.
      setFeedback({
        success: `Saved ${savedCodes} with CP ${formatMoney(res.totalCost)} and SP ${formatMoney(res.sellingPrice)}.${photo.message}`,
      });
      if (res.productId) details.setSelectedProductId(res.productId);
    });
  }

  return { save, isPending, feedback };
}
