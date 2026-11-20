"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useActionState, useMemo, useState } from "react";
import Link from "@/components/link";
import { saveHamper } from "./actions";
import { HamperTools } from "./_builder/hamper-tools";
import { ListReturnField, markRestore, useListHref } from "@/components/list-select/list-position";
import { priceHamper, num } from "@/lib/pricing";
import type { Category, Hamper, HamperItem, Settings } from "@/lib/types";
import { HamperInfo } from "./_builder/hamper-info";
import { HamperLineRow } from "./_builder/hamper-line-row";
import { HamperPricingStrip } from "./_builder/hamper-pricing";
import {
  blankLine,
  hamperPayload,
  initialFields,
  pricingOpts,
  productPatch,
  toLine,
  type CatalogProduct,
  type HamperFields,
  type Line,
} from "./_builder/hamper-state";

export type { CatalogProduct };

export function HamperBuilder({
  hamper,
  items,
  products,
  categories,
  settings,
  canEdit,
}: {
  hamper?: Hamper;
  items?: HamperItem[];
  products: CatalogProduct[];
  categories: Category[];
  settings: Settings;
  canEdit: boolean;
}) {
  const [state, action, saving] = useActionState(saveHamper, {});

  const [f, setFields] = useState<HamperFields>(() => initialFields(hamper));
  const set = (key: keyof HamperFields, value: string) => setFields((prev) => ({ ...prev, [key]: value }));
  const [lines, setLines] = useState<Line[]>(() => (items?.length ? items.map(toLine) : [blankLine()]));

  const countsAsItem = useMemo(() => {
    const map = new Map(categories.map((c) => [c.name.trim().toLowerCase(), c.counts_as_item]));
    // categoryCountsAsItem(): anything not listed in Settings counts.
    return (category: string) => map.get(category.trim().toLowerCase()) ?? true;
  }, [categories]);

  const totals = priceHamper(
    lines.map((l) => ({
      qty: num(l.qty),
      unitCp: num(l.unit_cp),
      unitSp: num(l.unit_sp),
      countsAsItem: countsAsItem(l.category_name),
    })),
    pricingOpts(f),
  );

  const update = (key: string, patch: Partial<Line>) =>
    setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));

  /** Swap a line with its neighbour. Line order is the printed order: the RPC
      numbers hamper_items by array position. */
  function move(key: string, delta: number) {
    setLines((prev) => {
      const i = prev.findIndex((l) => l.key === key);
      const j = i + delta;
      if (i < 0 || j < 0 || j >= prev.length) return prev;
      const next = [...prev];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }

  const listHref = useListHref("hampers");
  const filled = lines.filter((l) => l.product_name).length;

  return (
    <>
      <form action={action} onSubmit={() => markRestore("hampers")} className="space-y-4">
        <input type="hidden" name="payload" value={hamperPayload(f, lines, hamper)} />
        <ListReturnField kind="hampers" />

        <HamperInfo f={f} set={set} settings={settings} canEdit={canEdit} />
        <HamperPricingStrip f={f} set={set} totals={totals} canEdit={canEdit} />

        <section className="card overflow-hidden">
          <div className="flex items-center justify-between border-b border-(--color-line) px-4 py-2.5">
            <h2 className="text-sm font-semibold">
              Products
              <span className="ml-2 font-normal text-(--color-muted)">
                {filled} line{filled === 1 ? "" : "s"}
              </span>
            </h2>
            {canEdit && (
              <button type="button" className="btn-secondary" onClick={() => setLines((prev) => [...prev, blankLine()])}>
                Add line
              </button>
            )}
          </div>

          <div className="overflow-x-auto">
            <Table className="min-w-265">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-42.5">Category</TableHead>
                  <TableHead className="w-70">Product</TableHead>
                  <TableHead className="w-27.5">Code</TableHead>
                  <TableHead className="w-20 num">Qty</TableHead>
                  <TableHead className="w-27.5 num">Unit cost</TableHead>
                  <TableHead className="w-27.5 num">Total cost</TableHead>
                  <TableHead className="w-22.5 num">Margin %</TableHead>
                  <TableHead className="w-27.5 num">Unit price</TableHead>
                  <TableHead className="w-27.5 num">Total price</TableHead>
                  {canEdit && <TableHead className="w-22.5"></TableHead>}
                </TableRow>
              </TableHeader>
              <TableBody>
                {lines.map((line, index) => (
                  <HamperLineRow
                    key={line.key}
                    line={line}
                    products={products}
                    categories={categories}
                    canEdit={canEdit}
                    isFirst={index === 0}
                    isLast={index === lines.length - 1}
                    onChange={(patch) => update(line.key, patch)}
                    onSelectProduct={(id) => update(line.key, productPatch(products.find((p) => p.id === id)))}
                    onMove={(delta) => move(line.key, delta)}
                    onRemove={() =>
                      setLines((prev) => (prev.length === 1 ? [blankLine()] : prev.filter((l) => l.key !== line.key)))
                    }
                  />
                ))}
              </TableBody>
            </Table>
          </div>
        </section>

        {state.error && (
          <p role="alert" className="text-sm text-red-700">
            {state.error}
          </p>
        )}

        {canEdit && (
          <div className="flex flex-wrap items-center gap-2">
            <button type="submit" className="btn-primary" disabled={saving}>
              {saving ? "Saving…" : hamper ? "Save changes" : "Create hamper"}
            </button>
            <Link href={hamper ? `/hampers/${encodeURIComponent(hamper.code)}` : listHref} className="btn-secondary">
              Cancel
            </Link>
          </div>
        )}
      </form>

      {hamper && canEdit && <HamperTools hamper={hamper} />}
    </>
  );
}
