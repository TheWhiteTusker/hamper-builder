"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useActionState, useMemo, useState } from "react";
import Link from "@/components/link";
import { saveHamper, duplicateHamper, deleteHamper } from "./actions";
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
  const [dupState, dupAction, duplicating] = useActionState(duplicateHamper, {});
  const [delState, delAction, deleting] = useActionState(deleteHamper, {});

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

  const filled = lines.filter((l) => l.product_name).length;

  return (
    <>
      <form action={action} className="space-y-4">
        <input type="hidden" name="payload" value={hamperPayload(f, lines, hamper)} />

        <HamperInfo f={f} set={set} settings={settings} canEdit={canEdit} />
        <HamperPricingStrip f={f} set={set} totals={totals} canEdit={canEdit} />

        <section className="card overflow-hidden">
          <div className="flex items-center justify-between border-b border-[var(--color-line)] px-4 py-2.5">
            <h2 className="text-sm font-semibold">
              Products
              <span className="ml-2 font-normal text-[var(--color-muted)]">
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
            <Table className="min-w-[1060px]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[170px]">Category</TableHead>
                  <TableHead className="w-[280px]">Product</TableHead>
                  <TableHead className="w-[110px]">Code</TableHead>
                  <TableHead className="w-[80px] num">Qty</TableHead>
                  <TableHead className="w-[110px] num">Unit cost</TableHead>
                  <TableHead className="w-[110px] num">Total cost</TableHead>
                  <TableHead className="w-[90px] num">Margin %</TableHead>
                  <TableHead className="w-[110px] num">Unit price</TableHead>
                  <TableHead className="w-[110px] num">Total price</TableHead>
                  {canEdit && <TableHead className="w-[90px]"></TableHead>}
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

        {(state.error || dupState.error || delState.error) && (
          <p role="alert" className="text-sm text-red-700">
            {state.error || dupState.error || delState.error}
          </p>
        )}

        {canEdit && (
          <div className="flex flex-wrap items-center gap-2">
            <button type="submit" className="btn-primary" disabled={saving}>
              {saving ? "Saving…" : hamper ? "Save changes" : "Create hamper"}
            </button>
            <Link href="/hampers" className="btn-secondary">
              Cancel
            </Link>
          </div>
        )}
      </form>

      {/* Kept outside the builder form so these never submit the hamper. */}
      {hamper && canEdit && (
        <div className="mt-4 flex flex-wrap items-center gap-2">
          {hamper.image_url && (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={hamper.image_url}
              alt={`${hamper.name} image`}
              className="h-16 w-16 rounded-md border border-[var(--color-line)] object-cover"
            />
          )}
          {/* No prefetch: the image editor (and its canvas library) loads only when opened. */}
          <Link href={`/hampers/${encodeURIComponent(hamper.code)}/image`} prefetch={false} className="btn-secondary">
            {hamper.image_url ? "Edit image" : "Design image"}
          </Link>
          <Link href={`/quotes/new?hamper=${encodeURIComponent(hamper.code)}`} className="btn-secondary">
            Add to a quote
          </Link>

          <form action={dupAction}>
            <input type="hidden" name="code" value={hamper.code} />
            <button type="submit" className="btn-secondary" disabled={duplicating}>
              {duplicating ? "Duplicating…" : "Duplicate"}
            </button>
          </form>

          <form
            action={delAction}
            className="ml-auto"
            onSubmit={(e) => {
              if (!confirm(`Move ${hamper.name} to the Bin? It will stay in the Bin for 30 days and can be restored anytime.`)) {
                e.preventDefault();
              }
            }}
          >
            <input type="hidden" name="id" value={hamper.id} />
            <button type="submit" className="btn-danger" disabled={deleting}>
              {deleting ? "Moving to Bin…" : "Move to Bin"}
            </button>
          </form>
        </div>
      )}
    </>
  );
}
