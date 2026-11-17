"use client";

import { useId, useRef } from "react";
import { clearSelected, setSelected, useSelectedProducts } from "./selection";

/** Tick box at the start of a row; shown only while the product is selected. */
export function SelectBox({ id, label }: { id: string; label: string }) {
  const on = useSelectedProducts().includes(id);
  if (!on) return null;
  return (
    <input
      type="checkbox"
      checked
      onChange={() => setSelected(id, false)}
      aria-label={`Unselect ${label}`}
      className="h-4 w-4 cursor-pointer accent-(--color-brand)"
    />
  );
}

type DotsItem = { label: string; onClick: () => void; disabled?: boolean };

/**
 * A ⋮ button with a short list of actions. A native popover, so the table
 * card's overflow can't clip it and it closes on an outside click or Escape.
 */
function DotsMenu({ label, items, className }: { label: string; items: DotsItem[]; className: string }) {
  const popId = useId();
  const pop = useRef<HTMLDivElement>(null);

  return (
    <>
      <button
        type="button"
        popoverTarget={popId}
        aria-label={label}
        title={label}
        onClick={(e) => {
          // Pin it under the button; the popover itself opens after this.
          const r = e.currentTarget.getBoundingClientRect();
          const el = pop.current;
          if (el) {
            el.style.top = `${r.bottom + 4}px`;
            el.style.right = `${window.innerWidth - r.right}px`;
          }
        }}
        className={className}
      >
        ⋮
      </button>
      <div
        ref={pop}
        id={popId}
        popover="auto"
        className="fixed inset-auto m-0 w-40 rounded-lg border border-line bg-white p-1 text-left shadow-lg"
      >
        {items.map((i) => (
          <button
            key={i.label}
            type="button"
            disabled={i.disabled}
            onClick={() => {
              i.onClick();
              pop.current?.hidePopover();
            }}
            className="block w-full rounded-md px-3 py-2 text-left text-sm font-medium text-(--color-ink) hover:bg-paper focus:bg-paper focus:outline-none disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent"
          >
            {i.label}
          </button>
        ))}
      </div>
    </>
  );
}

/** ⋮ menu beside Edit / Delete. */
export function RowMenu({ id, label }: { id: string; label: string }) {
  const on = useSelectedProducts().includes(id);
  return (
    <DotsMenu
      label={`More actions for ${label}`}
      items={[{ label: on ? "Unselect" : "Select", onClick: () => setSelected(id, !on) }]}
      className="flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-lg leading-none text-(--color-muted) hover:bg-paper hover:text-(--color-ink)"
    />
  );
}

/** ⋮ menu in the page header, beside Excel / New product. */
export function ListMenu() {
  const n = useSelectedProducts().length;
  return (
    <DotsMenu
      label="More actions"
      items={[
        {
          label: n ? `Deselect all (${n})` : "Deselect all",
          disabled: n === 0,
          onClick: () => confirm(`Unselect ${n === 1 ? "the 1 selected product" : `all ${n} selected products`}?`) && clearSelected(),
        },
      ]}
      className="btn-secondary cursor-pointer px-3 text-base leading-5"
    />
  );
}

/** "3 selected · Clear" above the list; hidden when nothing is selected. */
export function SelectionCount() {
  const n = useSelectedProducts().length;
  if (n === 0) return null;
  return (
    <div className="mb-3 flex items-center gap-3 text-sm">
      <span className="font-medium text-(--color-ink)">
        {n} product{n === 1 ? "" : "s"} selected
      </span>
      <button
        type="button"
        onClick={() => confirm("Unselect all products?") && clearSelected()}
        className="font-medium text-(--color-brand) hover:underline"
      >
        Clear
      </button>
    </div>
  );
}
