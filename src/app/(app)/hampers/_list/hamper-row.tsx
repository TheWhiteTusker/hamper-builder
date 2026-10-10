import { TableCell, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { formatMoney, formatPct } from "@/lib/pricing";
import type { HamperSummary } from "@/lib/types";

/** One hamper in the list: picture, figures, and red where it loses money. */
export function HamperRow({ h }: { h: HamperSummary }) {
  return (
    <TableRow>
      <TableCell>
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="block h-10 w-10">
          {h.image_url ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={h.image_url}
              alt=""
              loading="lazy"
              className="h-10 w-10 rounded-md border border-[var(--color-line)] object-cover"
            />
          ) : (
            <span className="block h-10 w-10 rounded-md border border-dashed border-[var(--color-line)] bg-[var(--color-paper)]" />
          )}
        </Link>
      </TableCell>
      <TableCell className="font-mono whitespace-nowrap">
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.code}
        </Link>
      </TableCell>
      <TableCell>
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.name}
        </Link>
      </TableCell>
      <TableCell className="text-[var(--color-muted)]">{h.collection ?? "—"}</TableCell>
      <TableCell>
        <span className="badge">{h.status}</span>
      </TableCell>
      <TableCell className="num">{h.number_of_items}</TableCell>
      <TableCell className="num">{formatMoney(h.total_cp)}</TableCell>
      <TableCell className="num">
        {h.final_catalogue_sp == null ? (
          <span className="text-[var(--color-muted)]">not priced</span>
        ) : (
          formatMoney(h.final_catalogue_sp)
        )}
      </TableCell>
      <TableCell
        className={`num ${h.gross_profit != null && h.gross_profit < 0 ? "text-red-700" : ""}`}
      >
        {h.gross_profit == null ? "—" : formatMoney(h.gross_profit)}
      </TableCell>
      <TableCell
        className={`num ${h.final_margin != null && h.final_margin < 0 ? "text-red-700" : ""}`}
      >
        {h.final_margin == null ? "—" : formatPct(h.final_margin)}
      </TableCell>
    </TableRow>
  );
}
