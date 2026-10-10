"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { formatMoney, priceQuoteLine } from "@/lib/pricing";
import { HamperContents, type ContentItem } from "@/components/hamper-contents";
import type { Settings } from "@/lib/types";
import { pricedLine, type Line } from "./quote-state";

/** Choices from settings, keeping the line's current value if it was removed since. */
const choices = (list: string[], current: string) => Array.from(new Set([...list, current].filter(Boolean)));

export function QuoteLineRow({
  line,
  contents,
  settings,
  packagingCategories,
  canEdit,
  onChange,
  onRemove,
}: {
  line: Line;
  contents: ContentItem[];
  settings: Settings;
  packagingCategories: string[];
  canEdit: boolean;
  onChange: (patch: Partial<Line>) => void;
  onRemove: () => void;
}) {
  const { finalRate, amount } = priceQuoteLine(pricedLine(line));
  const num = (key: "qty" | "catalogue_price" | "discount_pct", label: string) => (
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
  const pick = (key: "detail_mode" | "packaging_treatment", label: string, list: string[]) => (
    <TableCell>
      <select
        aria-label={label}
        className="select"
        value={line[key]}
        disabled={!canEdit}
        onChange={(e) => onChange({ [key]: e.target.value })}
      >
        {choices(list, line[key]).map((m) => (
          <option key={m} value={m}>
            {m}
          </option>
        ))}
      </select>
    </TableCell>
  );

  return (
    <TableRow className="[&>td]:align-top">
      <TableCell>
        <input
          aria-label="Option label"
          className="input"
          value={line.option_label}
          disabled={!canEdit}
          onChange={(e) => onChange({ option_label: e.target.value })}
        />
      </TableCell>
      <TableCell>
        <div className="font-medium">{line.hamper_name}</div>
        <div className="font-mono text-xs text-[var(--color-muted)]">{line.hamper_code}</div>
        {/* Read-only preview of what the printed document will list for this line. */}
        <HamperContents
          className="text-xs"
          items={contents}
          detailMode={line.detail_mode}
          packagingTreatment={line.packaging_treatment}
          packagingCategories={packagingCategories}
        />
      </TableCell>
      {num("qty", "Quantity")}
      {num("catalogue_price", "Catalogue price")}
      {num("discount_pct", "Discount percent")}
      <TableCell className="num text-[var(--color-muted)]">{formatMoney(finalRate)}</TableCell>
      <TableCell className="num font-medium">{formatMoney(amount)}</TableCell>
      {/* Product lines have no hamper, so no contents or packaging to set. */}
      {line.hamper_id ? (
        <>
          {pick("detail_mode", "Contents shown", settings.detail_modes)}
          {pick("packaging_treatment", "Packaging treatment", settings.packaging_treatments)}
        </>
      ) : (
        <TableCell colSpan={2} className="text-xs text-[var(--color-muted)]">
          Single product
        </TableCell>
      )}
      {canEdit && (
        <TableCell className="num">
          <button
            type="button"
            aria-label={`Remove ${line.hamper_code}`}
            onClick={onRemove}
            className="rounded px-1.5 py-0.5 text-[var(--color-muted)] hover:bg-red-50 hover:text-red-700"
          >
            ×
          </button>
        </TableCell>
      )}
    </TableRow>
  );
}
