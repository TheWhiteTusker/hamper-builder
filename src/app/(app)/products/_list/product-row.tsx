import Link from "@/components/link";
import { formatMoney } from "@/lib/pricing";
import { ImagePreview } from "@/components/image-preview";
import type { ProductWithCategory } from "@/lib/types";
import { costingHref } from "../../cost-calculator/href";
import { productHref } from "../href";
import { DeleteProductButton } from "./delete-product-button";

/** One product in the Product Master table. */
export function ProductRow({ p, admin }: { p: ProductWithCategory; admin: boolean }) {
  return (
    <tr className={p.is_active ? "" : "opacity-55"}>
      <td className="w-12 py-1.5 px-2 text-center">
        {p.image_url ? (
          <ImagePreview
            src={p.image_url}
            alt={p.name}
            sizes="36px"
            className="mx-auto h-9 w-9 rounded-md border border-slate-200 bg-slate-100 shadow-2xs hover:ring-2 hover:ring-(--color-brand) transition-all"
          />
        ) : (
          <Link
            href={costingHref(p.code)}
            title={`Add photos for ${p.name}`}
            className="mx-auto flex h-9 w-9 items-center justify-center rounded-md border border-dashed border-slate-300 bg-slate-50 text-[10px] font-semibold text-slate-400 hover:border-(--color-brand) hover:text-(--color-brand) transition-colors"
          >
            + Pic
          </Link>
        )}
      </td>
      <td className="font-mono whitespace-nowrap">
        <Link
          href={productHref(p.code)}
          className="font-semibold text-(--color-ink) hover:text-(--color-brand) hover:underline"
        >
          {p.code}
        </Link>
      </td>
      <td>
        <Link
          href={productHref(p.code)}
          className="font-medium hover:underline text-(--color-ink)"
        >
          {p.name}
        </Link>
        {!p.is_active && <span className="badge ml-2">Inactive</span>}
      </td>
      <td className="text-(--color-muted)">{p.categories?.name ?? "—"}</td>
      <td className="text-(--color-muted)">{p.source ?? "—"}</td>
      <td className="num">{formatMoney(p.cost_price)}</td>
      <td className="num">{p.markup_pct != null ? `${p.markup_pct}%` : "—"}</td>
      <td className="num">{formatMoney(p.default_sp)}</td>
      <td className="num">
        <div className="flex items-center justify-end gap-2.5">
          {/* Editing a product happens in the cost calculator. */}
          {admin ? (
            <>
              <Link
                href={costingHref(p.code)}
                className="font-medium text-(--color-ink) hover:text-(--color-brand) hover:underline"
              >
                Edit
              </Link>
              <DeleteProductButton id={p.id} label={`${p.code} (${p.name})`} />
            </>
          ) : (
            <Link href={costingHref(p.code)} className="text-(--color-brand) hover:underline font-medium">
              Costing
            </Link>
          )}
        </div>
      </td>
    </tr>
  );
}
