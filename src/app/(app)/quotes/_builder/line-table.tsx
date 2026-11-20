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
      <p className="px-4 py-6 text-center text-sm text-(--color-muted)">
        Nothing added yet. Add a hamper or a single product above.
      </p>
    );
  }
  return (
    <div className="overflow-x-auto">
      <Table className="min-w-250">
        <TableHeader>
          <TableRow>
            <TableHead className="w-65">Item</TableHead>
            <TableHead className="w-20 num">Qty</TableHead>
            <TableHead className="w-30 num">Catalogue price</TableHead>
            <TableHead className="w-22.5 num">Discount %</TableHead>
            <TableHead className="w-30 num">Final rate</TableHead>
            <TableHead className="w-30 num">Amount</TableHead>
            <TableHead className="w-37.5">Contents shown</TableHead>
            <TableHead className="w-47.5">Packaging</TableHead>
            {canEdit && <TableHead className="w-10"></TableHead>}
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
