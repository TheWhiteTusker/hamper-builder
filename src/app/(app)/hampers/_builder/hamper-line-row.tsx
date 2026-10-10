"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { useMemo } from "react";
import Image from "next/image";
import { formatMoney, num } from "@/lib/pricing";
import type { Category } from "@/lib/types";
import { Combo, RowButton } from "./controls";
import { productLabel, type CatalogProduct, type Line } from "./hamper-state";
import { thumbProps } from "@/lib/product-images";

export function HamperLineRow({
  line,
  products,
  categories,
  canEdit,
  isFirst,
  isLast,
  onChange,
  onSelectProduct,
  onMove,
  onRemove,
}: {
  line: Line;
  products: CatalogProduct[];
  categories: Category[];
  canEdit: boolean;
  isFirst: boolean;
  isLast: boolean;
  onChange: (patch: Partial<Line>) => void;
  onSelectProduct: (productId: string) => void;
  onMove: (delta: number) => void;
  onRemove: () => void;
}) {
  // Filtering happens here in the browser. The spreadsheet rebuilt the dropdown
  // server-side on every category change, which is what made it feel slow.
  const options = useMemo(() => {
    if (!line.category_name) return products;
    const wanted = line.category_name.trim().toLowerCase();
    return products.filter((p) => (p.category_name ?? "").trim().toLowerCase() === wanted);
  }, [products, line.category_name]);

  const byLabel = useMemo(() => new Map(options.map((p) => [productLabel(p.name, p.code), p])), [options]);
  const allByLabel = useMemo(() => new Map(products.map((p) => [productLabel(p.name, p.code), p])), [products]);
  const categoryOptions = useMemo(() => ["All categories", ...categories.map((c) => c.name)], [categories]);

  const totalCp = num(line.qty) * num(line.unit_cp);
  const totalSp = num(line.qty) * num(line.unit_sp);
  const selectedProduct = products.find((p) => p.id === line.product_id);
  const what = line.product_name || "line";
  const numCell = (key: "qty" | "unit_cp" | "target_margin" | "unit_sp", label: string) => (
    <TableCell>
      <input
        aria-label={label}
        inputMode="decimal"
        className="input input-num"
        value={line[key]}
        disabled={!canEdit}
        onChange={(e) => onChange({ [key]: e.target.value })}
      />
    </TableCell>
  );

  return (
    <TableRow>
      <TableCell>
        <Combo
          label="Category"
          placeholder="All categories"
          value={line.category_name}
          options={categoryOptions}
          disabled={!canEdit}
          onPick={(category) => {
            const cat = category === "All categories" ? "" : category;
            const stillMatches = !cat || selectedProduct?.category_name === cat;
            // Clearing a mismatched product mirrors the sheet's onEdit handler.
            onChange(
              stillMatches
                ? { category_name: cat }
                : { category_name: cat, product_id: null, product_code: "", product_name: "" },
            );
          }}
        />
      </TableCell>

      <TableCell>
        <div className="flex items-center gap-2">
          {selectedProduct?.image_url && (
            <div className="relative h-8 w-8 shrink-0 overflow-hidden rounded-md border border-slate-200 bg-slate-100 shadow-2xs">
              <Image {...thumbProps(selectedProduct.image_url)} alt={selectedProduct.name} fill sizes="32px" className="object-cover" unoptimized />
            </div>
          )}
          <div className="min-w-0 flex-1">
            <Combo
              label="Product"
              placeholder="Search products…"
              value={line.product_name ? productLabel(line.product_name, line.product_code) : ""}
              options={Array.from(byLabel.keys())}
              allOptions={Array.from(allByLabel.keys())}
              disabled={!canEdit}
              onPick={(text) => {
                const p = byLabel.get(text) ?? allByLabel.get(text);
                if (p) onSelectProduct(p.id);
                else if (text === "") onSelectProduct("");
              }}
            />
            {/* A product that has since been retired still shows on saved lines. */}
            {!line.product_id && line.product_name && (
              <div className="mt-0.5 text-xs text-(--color-muted)">No longer in Product Master</div>
            )}
          </div>
        </div>
      </TableCell>

      <TableCell className="font-mono text-xs text-(--color-muted)">{line.product_code || "—"}</TableCell>
      {numCell("qty", "Quantity")}
      {numCell("unit_cp", "Unit cost")}
      <TableCell className="num text-(--color-muted)">{totalCp ? formatMoney(totalCp) : "—"}</TableCell>
      {numCell("target_margin", "Target margin")}
      {numCell("unit_sp", "Unit price")}
      <TableCell className="num font-medium">{totalSp ? formatMoney(totalSp) : "—"}</TableCell>

      {canEdit && (
        <TableCell>
          <div className="flex items-center justify-end gap-0.5">
            <RowButton label={`Move ${what} up`} disabled={isFirst} onClick={() => onMove(-1)}>
              ↑
            </RowButton>
            <RowButton label={`Move ${what} down`} disabled={isLast} onClick={() => onMove(1)}>
              ↓
            </RowButton>
            <RowButton label={`Remove ${what}`} onClick={onRemove} danger>
              ×
            </RowButton>
          </div>
        </TableCell>
      )}
    </TableRow>
  );
}
