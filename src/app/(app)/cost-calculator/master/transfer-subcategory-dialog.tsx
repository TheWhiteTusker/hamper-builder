"use client";

import { useId, useState } from "react";
import type { CostStageWithHierarchy, CostSubcategoryWithVarieties } from "@/lib/types";

export function TransferSubcategoryDialog({
  sub,
  initialMode,
  currentCategoryId,
  stages,
  isPending,
  onClose,
  onMove,
  onDuplicate,
}: {
  sub: CostSubcategoryWithVarieties;
  initialMode: "move" | "duplicate";
  currentCategoryId: string;
  stages: CostStageWithHierarchy[];
  isPending: boolean;
  onClose: () => void;
  onMove: (subcategoryId: string, targetCategoryId: string) => void;
  onDuplicate: (subcategoryId: string, targetCategoryId: string, newName: string) => void;
}) {
  const [mode, setMode] = useState<"move" | "duplicate">(initialMode);
  const destId = useId();
  const nameId = useId();

  // Find all available categories across all stages
  const allCategories = stages.flatMap((s) => s.categories);
  const otherCategories = allCategories.filter((c) => c.id !== currentCategoryId);

  // Initial destination:
  // For move: prefer another category. For duplicate: default to current category.
  const [targetCatId, setTargetCatId] = useState<string>(() => {
    if (initialMode === "move" && otherCategories.length > 0) {
      return otherCategories[0].id;
    }
    return currentCategoryId;
  });

  const [duplicateName, setDuplicateName] = useState<string>(() => {
    return targetCatId === currentCategoryId ? `${sub.name} (Copy)` : sub.name;
  });

  function handleModeChange(newMode: "move" | "duplicate") {
    setMode(newMode);
    if (newMode === "move" && targetCatId === currentCategoryId && otherCategories.length > 0) {
      setTargetCatId(otherCategories[0].id);
    }
  }

  function handleCategoryChange(newCatId: string) {
    setTargetCatId(newCatId);
    if (
      mode === "duplicate" &&
      (!duplicateName || duplicateName === sub.name || duplicateName === `${sub.name} (Copy)`)
    ) {
      setDuplicateName(newCatId === currentCategoryId ? `${sub.name} (Copy)` : sub.name);
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (mode === "move") {
      if (!targetCatId || targetCatId === currentCategoryId) return;
      onMove(sub.id, targetCatId);
    } else {
      if (!targetCatId || !duplicateName.trim()) return;
      onDuplicate(sub.id, targetCatId, duplicateName.trim());
    }
  }

  const currentCat = allCategories.find((c) => c.id === currentCategoryId);
  const isMoveInvalid = mode === "move" && (!targetCatId || targetCatId === currentCategoryId);
  const isDuplicateInvalid = mode === "duplicate" && (!targetCatId || !duplicateName.trim());

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4">
      <form
        onSubmit={handleSubmit}
        className="card max-w-lg w-full p-5 bg-white shadow-2xl space-y-4"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-(--color-line) pb-3">
          <div>
            <h3 className="text-base font-bold text-(--color-ink)">
              {mode === "move" ? "Move Subcategory" : "Duplicate Subcategory"}
            </h3>
            <p className="text-xs text-(--color-muted) mt-0.5">
              Source: <span className="font-semibold text-(--color-ink)">{sub.name}</span> in{" "}
              <span className="font-medium text-(--color-brand-dark)">{currentCat?.name ?? "Unknown"}</span>
            </p>
          </div>

          {/* Mode Switcher Tabs */}
          <div className="inline-flex rounded-lg bg-slate-100 p-1 text-xs font-medium">
            <button
              type="button"
              onClick={() => handleModeChange("move")}
              className={`rounded-md px-2.5 py-1 transition-colors ${
                mode === "move"
                  ? "bg-white font-bold text-sky-700 shadow-xs"
                  : "text-slate-600 hover:text-slate-900"
              }`}
            >
              Move
            </button>
            <button
              type="button"
              onClick={() => handleModeChange("duplicate")}
              className={`rounded-md px-2.5 py-1 transition-colors ${
                mode === "duplicate"
                  ? "bg-white font-bold text-indigo-700 shadow-xs"
                  : "text-slate-600 hover:text-slate-900"
              }`}
            >
              Duplicate
            </button>
          </div>
        </div>

        {/* Destination Category Picker */}
        <div>
          <label className="label" htmlFor={destId}>
            Destination Category *
          </label>
          <select
            id={destId}
            value={targetCatId}
            onChange={(e) => handleCategoryChange(e.target.value)}
            required
            className="select mt-1 w-full text-sm"
          >
            {stages.map((stage) => (
              <optgroup key={stage.id} label={`${stage.name} (${stage.categories.length})`}>
                {stage.categories.map((c) => (
                  <option
                    key={c.id}
                    value={c.id}
                    disabled={mode === "move" && c.id === currentCategoryId}
                  >
                    {c.name} {c.id === currentCategoryId ? "(Current Category)" : ""}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
          {isMoveInvalid && (
            <p className="mt-1 text-xs text-amber-600">
              Please choose a different category to move this subcategory to.
            </p>
          )}
        </div>

        {/* Duplicate Name Input */}
        {mode === "duplicate" && (
          <div>
            <label className="label" htmlFor={nameId}>
              New Subcategory Name *
            </label>
            <input
              id={nameId}
              type="text"
              value={duplicateName}
              onChange={(e) => setDuplicateName(e.target.value)}
              placeholder="Subcategory name"
              required
              className="input mt-1 w-full text-sm font-semibold"
              autoFocus
            />
          </div>
        )}

        {/* Varieties summary */}
        <div className="rounded-lg bg-slate-50 border border-slate-200 p-3 space-y-2">
          <div className="flex items-center justify-between text-xs">
            <span className="font-semibold text-slate-700">Varieties ({sub.varieties.length}):</span>
            <span className="text-[11px] text-(--color-muted)">
              {mode === "move"
                ? "Will be relocated with the subcategory"
                : "Will be duplicated with rates & specs"}
            </span>
          </div>
          {sub.varieties.length === 0 ? (
            <p className="text-xs text-(--color-muted) italic">No varieties in this subcategory.</p>
          ) : (
            <div className="flex flex-wrap gap-1.5 max-h-24 overflow-y-auto">
              {sub.varieties.map((v) => (
                <span
                  key={v.id}
                  className="inline-flex items-center rounded-md bg-white px-2 py-0.5 text-[11px] font-medium text-slate-700 border border-slate-200"
                >
                  {v.name}
                  <span className="ml-1 text-[10px] text-slate-400">({v.unit})</span>
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Dialog Actions */}
        <div className="flex items-center justify-end gap-2 pt-3 border-t border-(--color-line)">
          <button type="button" onClick={onClose} disabled={isPending} className="btn-secondary text-sm">
            Cancel
          </button>
          <button
            type="submit"
            disabled={isPending || isMoveInvalid || isDuplicateInvalid}
            className={`btn-primary text-sm ${
              mode === "duplicate" ? "bg-indigo-600 hover:bg-indigo-700 border-indigo-700" : ""
            }`}
          >
            {isPending
              ? mode === "move"
                ? "Moving…"
                : "Duplicating…"
              : mode === "move"
              ? "Move Subcategory"
              : "Duplicate Subcategory"}
          </button>
        </div>
      </form>
    </div>
  );
}
