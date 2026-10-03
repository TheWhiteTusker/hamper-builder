"use client";

import { useMemo, useState } from "react";
import { formatMoney } from "@/lib/pricing";
import type { DeckItem } from "./deck-schemas";
import { thumbProps } from "@/lib/product-images";

export type PickableItem = {
  type: "hamper" | "product";
  id: string;
  code: string;
  name: string;
  price: number | null;
  image_url: string | null;
};

/**
 * Tick hampers and products on the left; the right column is the slide order,
 * which the arrows rearrange.
 */
export function ItemPicker({
  hampers,
  products,
  value,
  onChange,
}: {
  hampers: PickableItem[];
  products: PickableItem[];
  value: DeckItem[];
  onChange: (items: DeckItem[]) => void;
}) {
  const [tab, setTab] = useState<"hamper" | "product">("hamper");
  const [q, setQ] = useState("");

  const byKey = useMemo(
    () => new Map([...hampers, ...products].map((i) => [`${i.type}:${i.id}`, i])),
    [hampers, products],
  );
  const picked = new Set(value.map((v) => `${v.type}:${v.id}`));

  const list = (tab === "hamper" ? hampers : products).filter((i) => {
    const s = q.trim().toLowerCase();
    return !s || i.name.toLowerCase().includes(s) || i.code.toLowerCase().includes(s);
  });

  const toggle = (i: PickableItem) => {
    const key = `${i.type}:${i.id}`;
    onChange(picked.has(key) ? value.filter((v) => `${v.type}:${v.id}` !== key) : [...value, { type: i.type, id: i.id }]);
  };

  const move = (index: number, by: -1 | 1) => {
    const next = [...value];
    const to = index + by;
    if (to < 0 || to >= next.length) return;
    [next[index], next[to]] = [next[to], next[index]];
    onChange(next);
  };

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
      <div className="card flex min-h-0 flex-col p-3">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          {(["hamper", "product"] as const).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => setTab(t)}
              className={tab === t ? "btn-primary" : "btn-secondary"}
            >
              {t === "hamper" ? `Hampers (${hampers.length})` : `Products (${products.length})`}
            </button>
          ))}
          <input
            className="input ml-auto max-w-xs"
            placeholder="Search name or code"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>

        <ul className="max-h-[480px] divide-y divide-[var(--color-line)] overflow-y-auto">
          {list.length === 0 && <li className="py-8 text-center text-sm text-[var(--color-muted)]">Nothing matches.</li>}
          {list.map((i) => {
            const on = picked.has(`${i.type}:${i.id}`);
            return (
              <li key={i.id}>
                <label className="flex cursor-pointer items-center gap-3 px-2 py-2 hover:bg-[var(--color-paper)]">
                  <input type="checkbox" checked={on} onChange={() => toggle(i)} className="h-4 w-4 accent-[var(--color-brand)]" />
                  {i.image_url ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img {...thumbProps(i.image_url)} alt="" loading="lazy" className="h-10 w-10 rounded-md border border-[var(--color-line)] object-cover" />
                  ) : (
                    <span className="h-10 w-10 rounded-md border border-dashed border-[var(--color-line)] bg-[var(--color-paper)]" />
                  )}
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">{i.name}</span>
                    <span className="font-mono text-xs text-[var(--color-muted)]">{i.code}</span>
                  </span>
                  <span className="text-sm tabular-nums text-[var(--color-muted)]">
                    {i.price == null ? "—" : formatMoney(i.price)}
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
      </div>

      <div className="card self-start p-3">
        <div className="label mb-2">Slide order ({value.length})</div>
        {value.length === 0 ? (
          <p className="py-6 text-center text-sm text-[var(--color-muted)]">Tick hampers or products to add their slides.</p>
        ) : (
          <ol className="space-y-1">
            {value.map((v, index) => {
              const item = byKey.get(`${v.type}:${v.id}`);
              return (
                <li key={`${v.type}:${v.id}`} className="flex items-center gap-2 rounded-md bg-[var(--color-paper)] px-2 py-1.5 text-sm">
                  <span className="w-5 text-right tabular-nums text-[var(--color-muted)]">{index + 1}</span>
                  <span className="min-w-0 flex-1 truncate">{item?.name ?? "Unknown"}</span>
                  <span className="badge">{v.type}</span>
                  <button type="button" aria-label="Move up" className="px-1 disabled:opacity-30" disabled={index === 0} onClick={() => move(index, -1)}>
                    ↑
                  </button>
                  <button
                    type="button"
                    aria-label="Move down"
                    className="px-1 disabled:opacity-30"
                    disabled={index === value.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    ↓
                  </button>
                  <button type="button" aria-label="Remove" className="px-1 text-red-700" onClick={() => item && toggle(item)}>
                    ×
                  </button>
                </li>
              );
            })}
          </ol>
        )}
      </div>
    </div>
  );
}

/** Loads everything the picker lists. */
export type PickerData = { hampers: PickableItem[]; products: PickableItem[] };
