"use client";

import { useState, useTransition } from "react";
import type { ProductColor } from "@/lib/product-code";
import type { CostStageWithHierarchy, CostSubcategoryWithVarieties, CostVariety } from "@/lib/types";
import { duplicateCostSubcategory, moveCostSubcategory, saveCostVariety } from "./actions";
import { ColorsCard } from "./colors-card";
import { filterStages } from "./filter";
import type { MasterCtx } from "./master-ui";
import { StageCard } from "./stage-card";
import { SyncRatesButton } from "./sync-rates-button";
import { TransferSubcategoryDialog } from "./transfer-subcategory-dialog";
import { VarietyDialog } from "./variety-dialog";

export function CostMasterView({
  stages,
  productColors,
}: {
  stages: CostStageWithHierarchy[];
  productColors: ProductColor[];
}) {
  const [isPending, startTransition] = useTransition();
  const [feedback, setFeedback] = useState<{ error?: string; success?: string }>({});
  // The variety being added or edited, and the subcategory it belongs to.
  const [editing, setEditing] = useState<{
    subcategoryId: string;
    variety: Partial<CostVariety>;
  } | null>(null);

  // The subcategory being moved or duplicated.
  const [transferModal, setTransferModal] = useState<{
    sub: CostSubcategoryWithVarieties;
    mode: "move" | "duplicate";
    currentCategoryId: string;
  } | null>(null);

  // Collapsed by default; a search expands everything it matched.
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const q = query.trim().toLowerCase();
  const visibleStages = filterStages(stages, q);

  const ctx: MasterCtx = {
    isPending,
    isOpen: (id) => q !== "" || expanded.has(id),
    toggle: (id) =>
      setExpanded((s) => {
        const next = new Set(s);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    expand: (id) => setExpanded((s) => new Set(s).add(id)),
    run: (action, success, after) => {
      setFeedback({});
      startTransition(async () => {
        const res = await action();
        if (res.error) return setFeedback({ error: res.error });
        after?.();
        setFeedback({ success: res.message ?? success });
      });
    },
    editVariety: (subcategoryId, variety) => setEditing({ subcategoryId, variety }),
    openTransferSubcategory: (sub, mode, currentCategoryId) =>
      setTransferModal({ sub, mode, currentCategoryId }),
  };

  return (
    <div className="space-y-6">
      <input
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search types, categories, subcategories, varieties, notes…"
        aria-label="Search the rate and hierarchy master"
        className="input text-sm"
      />

      <SyncRatesButton />

      <ColorsCard productColors={productColors} />

      {feedback.error && (
        <div
          role="alert"
          className="rounded-lg bg-red-50 p-3 text-sm font-medium text-red-700 border border-red-200"
        >
          {feedback.error}
        </div>
      )}
      {feedback.success && (
        <div
          role="status"
          className="rounded-lg bg-emerald-50 p-3 text-sm font-medium text-emerald-800 border border-emerald-200"
        >
          {feedback.success}
        </div>
      )}

      {q && visibleStages.length === 0 && (
        <p className="text-sm text-(--color-muted) italic">
          Nothing matches &ldquo;{query.trim()}&rdquo;.
        </p>
      )}
      {visibleStages.map((stage) => (
        <StageCard key={stage.id} stage={stage} ctx={ctx} />
      ))}

      {editing && (
        <VarietyDialog
          {...editing}
          isPending={isPending}
          onCancel={() => setEditing(null)}
          onSave={(fd) =>
            ctx.run(
              () => saveCostVariety(fd),
              "Variety / Specification saved successfully!",
              () => setEditing(null),
            )
          }
        />
      )}

      {transferModal && (
        <TransferSubcategoryDialog
          sub={transferModal.sub}
          initialMode={transferModal.mode}
          currentCategoryId={transferModal.currentCategoryId}
          stages={stages}
          isPending={isPending}
          onClose={() => setTransferModal(null)}
          onMove={(subId, targetCatId) =>
            ctx.run(
              () => moveCostSubcategory(subId, targetCatId),
              "Subcategory moved successfully!",
              () => {
                const targetCat = stages.flatMap((s) => s.categories).find((c) => c.id === targetCatId);
                setTransferModal(null);
                ctx.expand(targetCatId);
                ctx.expand(subId);
              },
            )
          }
          onDuplicate={(subId, targetCatId, newName) =>
            ctx.run(
              () => duplicateCostSubcategory(subId, targetCatId, newName),
              "Subcategory duplicated successfully!",
              () => {
                setTransferModal(null);
                ctx.expand(targetCatId);
              },
            )
          }
        />
      )}
    </div>
  );
}
