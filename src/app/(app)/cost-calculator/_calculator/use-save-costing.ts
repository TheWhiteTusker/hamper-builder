import { useState, useTransition } from "react";
import { formatMoney, num } from "@/lib/pricing";
import { roundUpToNext10 } from "@/lib/numbers.ts";
import type { Product, ProductCostSheet } from "@/lib/types";
import type { Sibling } from "./apply-price-dialog";
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
  initialProduct,
  siblings = [],
}: {
  details: ProductDetails;
  pricing: Pricing;
  photos: ProductPhotos;
  activeLines: LineState[];
  initialSheet?: ProductCostSheet | null;
  initialProduct?: Product | null;
  /** This product's other colours, offered the new price on save. */
  siblings?: Sibling[];
}) {
  const [isPending, startTransition] = useTransition();
  const [feedback, setFeedback] = useState<Feedback>({});
  const [asking, setAsking] = useState(false);

  // What the save dialog shows; null when there is nothing to ask.
  const newSp = roundUpToNext10(pricing.effectiveSp);
  const others = siblings.filter((s) => details.selectedColors.includes(s.color));
  const oldSp = num(initialProduct?.default_sp);
  const priceChanged =
    newSp !== oldSp || pricing.totals.total_cost !== num(initialProduct?.cost_price);
  const ask =
    initialProduct && others.length && priceChanged
      ? { code: initialProduct.code, color: initialProduct.colors?.[0] ?? "", oldSp, newSp, siblings: others }
      : null;

  function save() {
    setFeedback({});
    const code = details.code.trim();
    if (!code) return setFeedback({ error: "Product Code is required." });
    if (!details.name.trim()) return setFeedback({ error: "Product Name is required." });
    if (ask) return setAsking(true);
    commit([]);
  }

  function commit(applyToIds: string[]) {
    const code = details.code.trim();
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
        applyToIds,
      });
      setAsking(false);
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

  return { save, commit, isPending, feedback, ask: asking ? ask : null, cancelAsk: () => setAsking(false) };
}
