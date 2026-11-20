"use client";

import type { ProductColor } from "@/lib/product-code";
import { ProductImagesManager } from "@/components/product-images-manager";
import type {
  Category,
  CostStageWithHierarchy,
  Product,
  ProductCostLine,
  ProductCostSheet,
  ProductImage,
} from "@/lib/types";
import { ApplyPriceDialog, type Sibling } from "./apply-price-dialog";
import { CostSummary } from "./cost-summary";
import { productHref } from "../../products/href";
import { LeaveDialog, useLeaveGuard } from "./leave-guard";
import { BOUGHT_OUT, savedLines } from "./lines";
import { ProductDetailsCard } from "./product-details-card";
import { StageSection } from "./stage-section";
import { useCostLines } from "./use-cost-lines";
import { usePricing } from "./use-pricing";
import { useProductDetails } from "./use-product-details";
import { useProductPhotos } from "./use-product-photos";
import { useSaveCosting } from "./use-save-costing";

export function CostCalculatorView({
  stages,
  categories,
  productColors = [],
  initialProduct,
  initialSheet,
  initialImages = [],
  initialVariantIds = {},
  ownColor = "",
  siblings = [],
  savedCodes,
  gstRate = 0,
}: {
  stages: CostStageWithHierarchy[];
  categories: Category[];
  productColors?: ProductColor[];
  initialProduct?: Product | null;
  initialSheet?: (ProductCostSheet & { lines?: ProductCostLine[] }) | null;
  initialImages?: ProductImage[];
  /** Existing colour variant name -> sibling product id, used by photo uploads. */
  initialVariantIds?: Record<string, string>;
  /** The edited product's own colour. */
  ownColor?: string;
  /** The other colours of this product, with their current prices. */
  siblings?: Sibling[];
  /** Codes from the save that reset this page, shown as a banner. */
  savedCodes?: string;
  /** From Settings: a bought-out MRP includes GST, so the SP must leave room for it. */
  gstRate?: number;
}) {
  const details = useProductDetails({
    stages,
    categories,
    initialProduct,
    initialSheet,
    ownColor,
    siblingColors: siblings.map((s) => s.color),
  });
  const photos = useProductPhotos(initialImages, details.selectedProductId, initialVariantIds);
  const api = useCostLines(stages, initialSheet?.lines);

  // All lines across all stages count towards product costing
  const activeLines = api.lines;
  const pricing = usePricing(activeLines, initialProduct, initialSheet, gstRate);
  const saving = useSaveCosting({ details, pricing, photos, activeLines, initialSheet, initialProduct, siblings });
  const { save, isPending, feedback } = saving;
  const { selectedProductId, selectedColors } = details;
  // Everything a save would send (the code follows category and colour).
  const leave = useLeaveGuard(
    JSON.stringify([
      details.name, details.categoryId, details.source, selectedColors, details.isActive, details.notes,
      pricing.markupPct, pricing.manualSp, pricing.stageOverheads, savedLines(activeLines), photos.pending.length,
    ]),
    initialProduct ? productHref(initialProduct.code) : "/products",
  );

  return (
    <div className="space-y-6">
      {initialProduct && (
        <button
          type="button"
          onClick={leave.back}
          className="-mb-3 inline-flex items-center gap-1 text-sm font-medium text-(--color-muted) hover:text-(--color-brand)"
        >
          <span aria-hidden>←</span> Back
        </button>
      )}
      {leave.asking && (
        <LeaveDialog isPending={isPending} error={feedback.error} onSave={save} onDiscard={leave.discard} onCancel={leave.cancel} />
      )}
      {savedCodes && (
        <div className="rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-2.5 text-sm font-medium text-emerald-800">
          ✓ Saved <span className="font-mono">{savedCodes}</span>. Ready for the next product.
        </div>
      )}

      <ProductDetailsCard
        details={details}
        photos={photos}
        siblings={siblings}
        categories={categories}
        productColors={productColors}
        imageUrl={initialProduct?.image_url}
      />

      {selectedProductId ? (
        <div className="card p-5 shadow-sm">
          <ProductImagesManager
            // Remount after a colour-row upload so the gallery shows it
            key={`${selectedProductId}-${photos.photoVersion}`}
            productId={selectedProductId}
            // Every colour's photos, including those saved on sibling colour products
            initialImages={photos.images}
            onImagesChange={photos.handleGalleryChange}
            canUpload={false}
            productName={details.name}
            currentColor={selectedColors[0]}
            colors={
              selectedColors.length
                ? productColors.filter((c) => selectedColors.includes(c.name))
                : productColors
            }
          />
        </div>
      ) : (
        <div className="card p-4 border-dashed bg-slate-50/60 text-center text-xs text-(--color-muted)">
          Save this product first to upload and manage photos for each of its color finishes.
        </div>
      )}

      {stages.map((stage) => (
        <StageSection
          key={stage.id}
          stage={stage}
          api={api}
          collapsed={details.collapsedStages.has(stage.code)}
          onToggle={() => details.toggleStage(stage.code)}
          overhead={pricing.overheadFor(stage.code)}
          onOverhead={(pct) => pricing.setOverhead(stage.code, pct)}
          // Outsourced: bought-out items are typed in and saved to the master.
          freeEntry={stage.code === BOUGHT_OUT && details.source === "Outsource"}
          gstRate={gstRate}
        />
      ))}

      <CostSummary
        details={details}
        pricing={pricing}
        feedback={feedback}
        isPending={isPending}
        onSave={save}
      />
      {saving.ask && (
        <ApplyPriceDialog {...saving.ask} isPending={isPending} onCancel={saving.cancelAsk} onSave={saving.commit} />
      )}
    </div>
  );
}
