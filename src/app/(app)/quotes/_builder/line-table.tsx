"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { ContentItem } from "@/components/hamper-contents";
import type { Settings } from "@/lib/types";
import { QuoteLineRow } from "./quote-line-row";
import type { Line } from "./quote-state";

/** The editable lines of a combined order, or of one option. */
export function LineTable({
  lines,
  contentsOf,
  settings,
  packagingCategories,
  canEdit,
  onChange,
  onRemove,
}: {
  lines: Line[];
  contentsOf: (hamperId: string | null) => ContentItem[];
  settings: Settings;
  packagingCategories: string[];
  canEdit: boolean;
  onChange: (key: string, patch: Partial<Line>) => void;
  onRemove: (key: string) => void;
}) {
  if (!lines.length) {
    return (
      <p className="px-4 py-6 text-center text-sm text-[var(--color-muted)]">
        Nothing added yet. Add a hamper or a single product above.
      </p>
    );
  }
  return (
    <div className="overflow-x-auto">
      <Table className="min-w-[1000px]">
        <TableHeader>
          <TableRow>
            <TableHead className="w-[260px]">Item</TableHead>
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
  );
}
