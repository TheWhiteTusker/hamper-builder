"use client";

import { useState } from "react";
import type { CostSubcategoryWithVarieties, CostVariety } from "@/lib/types";

/** What every level of the hierarchy needs from the master view. */
export type MasterCtx = {
  isPending: boolean;
  isOpen: (id: string) => boolean;
  toggle: (id: string) => void;
  expand: (id: string) => void;
  /** Runs a server action, showing its error, its message, or `success` in the banner. */
  run: (action: () => Promise<{ error?: string; message?: string }>, success: string, after?: () => void) => void;
  editVariety: (subcategoryId: string, variety: Partial<CostVariety>) => void;
  openTransferSubcategory: (
    sub: CostSubcategoryWithVarieties,
    mode: "move" | "duplicate",
    currentCategoryId: string,
  ) => void;
};

export { Chevron } from "@/components/ui";

/** Inline name box for adding a category or subcategory. */
export function InlineAdd({
  placeholder,
  saveLabel,
  busy,
  className,
  onSave,
  onCancel,
}: {
  placeholder: string;
  saveLabel: string;
  busy: boolean;
  className: string;
  onSave: (name: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState("");
  return (
    <div
      className={`rounded-lg border border-(--color-brand) p-3 flex flex-wrap items-center gap-3 ${className}`}
    >
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder}
        className="input text-xs py-1.5 max-w-xs"
        autoFocus
      />
      <button
        type="button"
        onClick={() => onSave(value.trim())}
        disabled={busy || !value.trim()}
        className="btn-primary text-xs py-1.5"
      >
        {saveLabel}
      </button>
      <button type="button" onClick={onCancel} className="btn-secondary text-xs py-1.5">
        Cancel
      </button>
    </div>
  );
}
