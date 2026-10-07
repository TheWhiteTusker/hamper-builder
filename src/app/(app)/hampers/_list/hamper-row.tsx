import Link from "@/components/link";
import { formatMoney, formatPct } from "@/lib/pricing";
import type { HamperSummary } from "@/lib/types";

/** One hamper in the list: picture, figures, and red where it loses money. */
export function HamperRow({ h }: { h: HamperSummary }) {
  return (
    <tr>
      <td>
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
      </td>
      <td className="font-mono whitespace-nowrap">
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.code}
        </Link>
      </td>
      <td>
        <Link href={`/hampers/${encodeURIComponent(h.code)}`} className="hover:underline">
          {h.name}
        </Link>
      </td>
      <td className="text-[var(--color-muted)]">{h.collection ?? "—"}</td>
      <td>
        <span className="badge">{h.status}</span>
      </td>
      <td className="num">{h.number_of_items}</td>
      <td className="num">{formatMoney(h.total_cp)}</td>
      <td className="num">
        {h.final_catalogue_sp == null ? (
          <span className="text-[var(--color-muted)]">not priced</span>
        ) : (
          formatMoney(h.final_catalogue_sp)
        )}
      </td>
      <td
        className={`num ${h.gross_profit != null && h.gross_profit < 0 ? "text-red-700" : ""}`}
      >
        {h.gross_profit == null ? "—" : formatMoney(h.gross_profit)}
      </td>
      <td
        className={`num ${h.final_margin != null && h.final_margin < 0 ? "text-red-700" : ""}`}
      >
        {h.final_margin == null ? "—" : formatPct(h.final_margin)}
      </td>
    </tr>
  );
}
