"use client";

import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";
import { PICK_LIST_CONFIGS, type PickListConfig, type PickListKey } from "./configs";

const values = (n: number) => `${n} ${n === 1 ? "value" : "values"}`;

/** Dropdown of every pick-list, with its size; the selected one is ticked. */
export function ListSelector({
  active,
  counts,
  onSelect,
}: {
  active: PickListConfig;
  counts: Record<PickListKey, number>;
  onSelect: (key: PickListKey) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div ref={ref} className="relative flex-1">
      <button
        type="button"
        id="picklist-selector"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-labelledby="picklist-label picklist-selector"
        onClick={() => setOpen((prev) => !prev)}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            setOpen(true);
          } else if (e.key === "Escape") {
            setOpen(false);
          }
        }}
        className="select flex w-full items-center justify-between font-medium cursor-pointer text-left"
      >
        <span className="truncate">
          {active.label} ({values(counts[active.key])})
        </span>
        <ChevronDown
          className={`h-4 w-4 shrink-0 text-(--color-muted) transition-transform duration-150 ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <div
          role="listbox"
          aria-labelledby="picklist-label"
          className="absolute left-0 top-full z-40 mt-1 max-h-72 w-full overflow-y-auto rounded-lg border border-(--color-line) bg-white py-1 shadow-lg"
        >
          {PICK_LIST_CONFIGS.map((config) => {
            const isSelected = config.key === active.key;
            return (
              <button
                key={config.key}
                type="button"
                role="option"
                aria-selected={isSelected}
                onClick={() => {
                  onSelect(config.key);
                  setOpen(false);
                }}
                className={`group flex w-full select-none items-center justify-between px-3 py-2 text-left text-sm cursor-pointer text-(--color-ink) hover:bg-(--color-brand) hover:text-white focus:bg-(--color-brand) focus:text-white focus:outline-none ${
                  isSelected ? "font-semibold" : "font-medium"
                }`}
              >
                <span className="truncate">
                  {config.label} ({values(counts[config.key])})
                </span>
                {isSelected && (
                  <Check
                    strokeWidth={3.2}
                    className="h-4.5 w-4.5 shrink-0 text-(--color-brand) group-hover:text-white group-focus:text-white"
                  />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
