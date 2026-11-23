"use client";

import { useEffect, type RefObject } from "react";
import { createPortal } from "react-dom";
import { Check } from "lucide-react";
import type { MenuCoords } from "./use-floating-menu";

/** "Name — CODE" shows the code muted on the right. */
function OptionLabel({ opt }: { opt: string }) {
  const dashIdx = opt.lastIndexOf(" — ");
  if (dashIdx <= 0) return <span className="truncate pr-1">{opt}</span>;
  return (
    <div className="flex items-center justify-between w-full min-w-0 pr-1">
      <span className="truncate">{opt.slice(0, dashIdx)}</span>
      <span className="ml-2 shrink-0 font-mono text-[11px] text-(--color-muted) font-normal">{opt.slice(dashIdx + 3)}</span>
    </div>
  );
}

/** The combo box's open list, in a portal so table overflow never clips it. */
export function ComboMenu({
  menuRef,
  coords,
  options,
  value,
  highlightedIndex,
  onSelect,
  onHighlight,
}: {
  menuRef: RefObject<HTMLDivElement | null>;
  coords: MenuCoords;
  options: string[];
  value: string;
  highlightedIndex: number;
  onSelect: (option: string) => void;
  onHighlight: (index: number) => void;
}) {
  // Keep the highlighted row in view.
  useEffect(() => {
    if (highlightedIndex < 0 || !menuRef.current) return;
    const el = menuRef.current.querySelector(`[data-index="${highlightedIndex}"]`) as HTMLElement | null;
    el?.scrollIntoView({ block: "nearest" });
  }, [highlightedIndex, menuRef]);

  return createPortal(
    <div
      ref={menuRef}
      style={{ position: "fixed", top: coords.top, left: coords.left, width: coords.width, transform: coords.transform, zIndex: 9999 }}
      className="max-h-60 overflow-y-auto rounded-lg border border-(--color-line) bg-white py-1 shadow-xl text-xs sm:text-sm"
    >
      {options.length === 0 ? (
        <div className="px-3 py-3 text-center text-xs text-(--color-muted)">No matching options</div>
      ) : (
        options.map((opt, idx) => {
          const isSelected = opt === value || (value === "" && opt === "All categories");
          const isHighlighted = idx === highlightedIndex;
          return (
            <button
              key={opt}
              type="button"
              data-index={idx}
              onClick={() => onSelect(opt)}
              onMouseEnter={() => onHighlight(idx)}
              className={`flex w-full select-none items-center justify-between px-3 py-1.5 text-left cursor-pointer transition-colors ${
                isHighlighted
                  ? "bg-paper text-(--color-ink)"
                  : isSelected
                  ? "bg-slate-50 text-(--color-ink) font-semibold"
                  : "text-(--color-ink)"
              }`}
            >
              <OptionLabel opt={opt} />
              {isSelected && <Check className="h-3.5 w-3.5 shrink-0 text-(--color-brand) ml-1" />}
            </button>
          );
        })
      )}
    </div>,
    document.body,
  );
}
