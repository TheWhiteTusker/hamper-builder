"use client";

import Link from "@/components/link";
import { productHref } from "../../products/href";
import { CostBreakdown } from "./cost-breakdown";
import { PricingBoxes } from "./pricing-boxes";
import type { Pricing } from "./use-pricing";
import type { ProductDetails } from "./use-product-details";
import type { Feedback } from "./use-save-costing";

/** Breakdown, pricing, notes and the Save button. */
export function CostSummary({
  details,
  pricing,
  feedback,
  isPending,
  onSave,
}: {
  details: ProductDetails;
  pricing: Pricing;
  feedback: Feedback;
  isPending: boolean;
  onSave: () => void;
}) {
  return (
    <div className="card p-5 bg-linear-to-br from-white to-slate-50 border-2 border-(--color-brand)/20 shadow-md">
      <h3 className="text-base font-bold text-(--color-ink) mb-4">
        Cost Breakdown & Selling Price Master
      </h3>

      <CostBreakdown totals={pricing.totals} />
      <PricingBoxes pricing={pricing} source={details.source} />

      <div className="mt-4">
        <label className="label" htmlFor="calc-notes">
          Costing Notes / Specifications
        </label>
        <input
          id="calc-notes"
          value={details.notes}
          onChange={(e) => details.setNotes(e.target.value)}
          placeholder="e.g. Requires 2 coats of PU polish, double-pass laser engraving on lid"
          className="input mt-1 text-sm"
        />
      </div>

      {feedback.error && (
        <div
          role="alert"
          className="mt-4 rounded-lg bg-red-50 p-3 text-sm font-medium text-red-700 border border-red-200"
        >
          {feedback.error}
        </div>
      )}
      {feedback.success && (
        <div
          role="status"
          className="mt-4 rounded-lg bg-emerald-50 p-3 text-sm font-medium text-emerald-800 border border-emerald-200"
        >
          {feedback.success}
        </div>
      )}

      <div className="mt-6 flex flex-wrap items-center justify-between gap-4 border-t border-(--color-border) pt-4">
        <div className="flex items-center gap-2">
          <Link href="/products" className="btn-secondary">
            &larr; Back to Products
          </Link>
          {details.selectedProductId && (
            <Link href={productHref(details.code)} className="btn-secondary">
              View in Product Master
            </Link>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={onSave}
            disabled={isPending}
            className="btn-primary px-5 py-2.5 text-sm font-bold shadow"
          >
            {isPending ? "Saving Costing…" : `Save Product (${details.code || "Primary"})`}
          </button>
        </div>
      </div>
    </div>
  );
}
