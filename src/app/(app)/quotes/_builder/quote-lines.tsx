"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useMemo } from "react";
import type { Settings } from "@/lib/types";
import { QuoteLineRow } from "./quote-line-row";
import type { HamperOption, Line, ProductOption } from "./quote-state";

/** The structure, the hamper / product pickers and the line table. */
export function QuoteLines({
  lines,
  structure,
  isCombined,
  hampers,
  products,
  settings,
  packagingCategories,
  canEdit,
  onStructure,
  onAddHamper,
  onAddProduct,
  onChange,
  onRemove,
}: {
  lines: Line[];
  structure: string;
  isCombined: boolean;
  hampers: HamperOption[];
  products: ProductOption[];
  settings: Settings;
  packagingCategories: string[];
  canEdit: boolean;
  onStructure: (s: string) => void;
  onAddHamper: (id: string) => void;
  onAddProduct: (id: string) => void;
  onChange: (key: string, patch: Partial<Line>) => void;
  onRemove: (key: string) => void;
}) {
  const alreadyAdded = useMemo(() => new Set(lines.map((l) => l.hamper_code)), [lines]);
  const contentsOf = useMemo(() => {
    const map = new Map(hampers.map((h) => [h.id, h.items]));
    return (hamperId: string | null) => (hamperId && map.get(hamperId)) || [];
  }, [hampers]);

  const picker = (id: string, label: string, placeholder: string, list: { id: string; code: string; name: string }[], onPick: (id: string) => void) => (
    <div>
      <label className="label" htmlFor={id}>
        {label}
      </label>
      {/* Always shows the placeholder: picking adds a line, then resets. */}
      <select id={id} className="select mt-1 min-w-[240px]" value="" onChange={(e) => onPick(e.target.value)}>
        <option value="">{placeholder}</option>
        {list.map((h) => (
          <option key={h.id} value={h.id}>
            {h.code} — {h.name}
            {alreadyAdded.has(h.code) ? " (already added)" : ""}
          </option>
        ))}
      </select>
    </div>
  );

  return (
    <section className="card overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-line)] px-4 py-2.5">
        <div>
          <h2 className="text-sm font-semibold">Hampers & products</h2>
          <p className="text-xs text-[var(--color-muted)]">
            {isCombined
              ? "Combined order — every line is part of one total."
              : "Option based — the client picks one line, so there is no order total."}
          </p>
        </div>

        <div className="flex items-end gap-2">
          <div>
            <label className="label" htmlFor="structure">
              Structure
            </label>
            <select
              id="structure"
              className="select mt-1"
              value={structure}
              onChange={(e) => onStructure(e.target.value)}
              disabled={!canEdit}
            >
              {Array.from(new Set([...settings.quote_structures, structure])).map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </div>
          {canEdit && picker("picker", "Add hamper", "Select a hamper…", hampers, onAddHamper)}
          {canEdit && picker("productPicker", "Add product", "Select a product…", products, onAddProduct)}
        </div>
      </div>

      {lines.length === 0 ? (
        <p className="px-4 py-8 text-center text-sm text-[var(--color-muted)]">
          Nothing added yet. Add a hamper or a single product above to start.
        </p>
      ) : (
        <div className="overflow-x-auto">
          <Table className="min-w-[1100px]">
            <TableHeader>
              <TableRow>
                <TableHead className="w-[110px]">Option</TableHead>
                <TableHead className="w-[240px]">Item</TableHead>
                <TableHead className="w-[80px] num">Qty</TableHead>
                <TableHead className="w-[120px] num">Catalogue price</TableHead>
                <TableHead className="w-[90px] num">Discount %</TableHead>
                <TableHead className="w-[120px] num">Final rate</TableHead>
                <TableHead className="w-[120px] num">Amount</TableHead>
                <TableHead className="w-[150px]">Contents shown</TableHead>
                <TableHead className="w-[190px]">Packaging</TableHead>
                {canEdit && <TableHead className="w-[40px]"></TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {lines.map((line) => (
                <QuoteLineRow
                  key={line.key}
                  line={line}
                  contents={contentsOf(line.hamper_id)}
                  settings={settings}
                  packagingCategories={packagingCategories}
                  canEdit={canEdit}
                  onChange={(patch) => onChange(line.key, patch)}
                  onRemove={() => onRemove(line.key)}
                />
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </section>
  );
}
