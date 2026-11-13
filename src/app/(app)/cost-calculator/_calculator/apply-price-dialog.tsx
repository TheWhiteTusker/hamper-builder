"use client";

import { useEffect, useRef, useState } from "react";
import { formatMoney } from "@/lib/pricing";

export type Sibling = { id: string; code: string; color: string; sp: number };

/**
 * Asked on saving a product that has other colours: shows this colour's old and
 * new price, and lets the new price (and its costing) go to the other colours too.
 */
export function ApplyPriceDialog({
  code,
  color,
  oldSp,
  newSp,
  siblings,
  isPending,
  onCancel,
  onSave,
}: {
  code: string;
  color: string;
  oldSp: number;
  newSp: number;
  siblings: Sibling[];
  isPending: boolean;
  onCancel: () => void;
  onSave: (ids: string[]) => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  useEffect(() => ref.current?.showModal(), []);

  const all = picked.size === siblings.length;
  const toggle = (id: string) =>
    setPicked((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  return (
    <dialog
      ref={ref}
      onClose={onCancel}
      aria-labelledby="apply-price-title"
      className="m-auto w-[min(92vw,30rem)] rounded-xl bg-white p-5 shadow-2xl backdrop:bg-black/50"
    >
      <h2 id="apply-price-title" className="text-base font-semibold">
        Price changed
      </h2>
      <p className="mt-2 text-sm">
        <span className="font-mono font-semibold">{code}</span> ({color}) was{" "}
        <span className="font-mono">{formatMoney(oldSp)}</span>, new price{" "}
        <span className="font-mono font-semibold text-(--color-brand-dark)">{formatMoney(newSp)}</span>.
      </p>

      <p className="mt-4 text-xs font-semibold text-(--color-muted)">Use the new price for these colours too:</p>
      <ul className="mt-2 divide-y divide-(--color-line) rounded-lg border border-(--color-line)">
        <li>
          <label className="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-sm font-semibold">
            <input
              type="checkbox"
              checked={all}
              onChange={() => setPicked(all ? new Set() : new Set(siblings.map((s) => s.id)))}
            />
            <span className="flex-1">All colours</span>
            <span className="text-[11px] font-semibold text-(--color-muted)">Previous SP</span>
          </label>
        </li>
        {siblings.map((s) => (
          <li key={s.id}>
            <label className="flex cursor-pointer items-center gap-2.5 px-3 py-2 text-sm">
              <input type="checkbox" checked={picked.has(s.id)} onChange={() => toggle(s.id)} />
              <span className="flex-1">
                {s.color} <span className="font-mono text-xs text-(--color-muted)">{s.code}</span>
              </span>
              <span className="whitespace-nowrap font-mono text-xs">
                <span className={picked.has(s.id) && s.sp !== newSp ? "text-(--color-muted) line-through" : ""}>
                  {formatMoney(s.sp)}
                </span>
                {picked.has(s.id) && s.sp !== newSp && (
                  <span className="font-semibold text-(--color-brand-dark)"> → {formatMoney(newSp)}</span>
                )}
              </span>
            </label>
          </li>
        ))}
      </ul>
      <p className="mt-2 text-[11px] text-(--color-muted)">
        Ticked colours get this whole costing: cost lines, CP and SP.
      </p>

      <div className="mt-5 flex justify-end gap-2">
        <button type="button" onClick={() => ref.current?.close()} className="btn-secondary" disabled={isPending}>
          Cancel
        </button>
        <button type="button" onClick={() => onSave([...picked])} className="btn-primary" disabled={isPending}>
          {isPending ? "Saving…" : picked.size ? `Save ${picked.size + 1} colours` : "Save this colour only"}
        </button>
      </div>
    </dialog>
  );
}
