import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { HamperContents } from "@/components/hamper-contents";
import { formatMoney } from "@/lib/pricing";
import type { HamperItem, QuoteItem } from "@/lib/types";

/** Printed lines of a combined order, or of one option. */
export function LinesTable({
  lines,
  byHamper,
  packagingCategories,
}: {
  lines: QuoteItem[];
  byHamper: Map<string, HamperItem[]>;
  packagingCategories: string[];
}) {
  return (
    <Table className="mt-3">
      <TableHeader>
        <TableRow>
          <TableHead className="w-8">#</TableHead>
          <TableHead>Item</TableHead>
          <TableHead className="num w-16">Qty</TableHead>
          <TableHead className="num w-24">Rate</TableHead>
          <TableHead className="num w-24">Amount</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {lines.map((line, index) => (
          <TableRow key={line.id} className="avoid-break align-top">
            <TableCell className="text-[var(--color-muted)]">{index + 1}</TableCell>
            <TableCell>
              <div className="font-medium">{line.hamper_name}</div>
              <HamperContents
                items={line.hamper_id ? (byHamper.get(line.hamper_id) ?? []) : []}
                detailMode={line.detail_mode}
                packagingTreatment={line.packaging_treatment}
                packagingCategories={packagingCategories}
              />
            </TableCell>
            <TableCell className="num">{line.qty}</TableCell>
            <TableCell className="num">
              {formatMoney(line.final_rate)}
              {Number(line.discount_pct) > 0 && (
                <div className="text-xs text-[var(--color-muted)] line-through">{formatMoney(line.catalogue_price)}</div>
              )}
            </TableCell>
            <TableCell className="num font-medium">{formatMoney(line.amount)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
