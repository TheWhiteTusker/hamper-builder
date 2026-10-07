import Link from "@/components/link";
import { productHref } from "../../href";
import { ImagePreview } from "@/components/image-preview";
import { codesForColors, type ProductColor } from "@/lib/product-code";
import type { Product, ProductImage } from "@/lib/types";

/** Code, category, origin, colours and photos, as the calculator's top card shows them. */
export function ProductCard({
  product,
  categoryName,
  colors,
  photos,
}: {
  product: Product;
  categoryName: string | null;
  colors: ProductColor[];
  photos: ProductImage[];
}) {
  const fields: [string, React.ReactNode][] = [
    ["Product code", <span key="c" className="font-mono font-semibold">{product.code}</span>],
    ["Category", categoryName ?? "—"],
    ["Product name", product.name],
    ["Product origin", product.source ?? "—"],
  ];

  return (
    <div className="card p-5">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {fields.map(([label, value]) => (
          <div key={label}>
            <div className="label">{label}</div>
            <div className="mt-1 text-sm text-(--color-ink)">{value}</div>
          </div>
        ))}
      </div>

      <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-(--color-border) pt-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs font-semibold text-(--color-ink)">Color finishes:</span>
          {colors.length === 0 && <span className="text-xs text-(--color-muted)">—</span>}
          {colors.map((c) => {
            const siblingCode = codesForColors(product.code, [c.name])[0]?.code ?? product.code;
            const isCurrent = siblingCode === product.code;
            return isCurrent ? (
              <span
                key={c.name}
                className="flex items-center gap-1.5 rounded-full bg-(--color-brand) px-2.5 py-1 text-xs font-bold text-white shadow-xs"
              >
                <span className="h-2.5 w-2.5 rounded-full border border-white/40" style={{ backgroundColor: c.hex }} />
                {c.name} ({c.code})
              </span>
            ) : (
              <Link
                key={c.name}
                href={productHref(siblingCode)}
                className="flex items-center gap-1.5 rounded-full bg-(--color-sheet) px-2.5 py-1 text-xs font-medium hover:bg-slate-200 transition-colors"
                title={`View ${c.name} variant`}
              >
                <span className="h-2.5 w-2.5 rounded-full border border-black/20" style={{ backgroundColor: c.hex }} />
                {c.name} ({c.code})
              </Link>
            );
          })}
        </div>
        <span className={`badge ${product.is_active ? "" : "opacity-70"}`}>
          {product.is_active ? "Active in Product Master & Hamper Builder" : "Inactive"}
        </span>
      </div>

      {photos.length > 0 && (
        <div className="mt-4 flex flex-wrap gap-3 border-t border-(--color-border) pt-4">
          {photos.map((img) => (
            <figure key={img.id} className="w-24">
              <ImagePreview
                src={img.url}
                alt={`${product.name}${img.color ? ` — ${img.color}` : ""}`}
                sizes="96px"
                className="aspect-square w-full rounded-lg border border-slate-200 bg-slate-100"
              />
              <figcaption className="mt-1 truncate text-[11px] text-(--color-muted)">
                {img.color || "General"}
                {img.is_primary && " · ★"}
              </figcaption>
            </figure>
          ))}
        </div>
      )}
    </div>
  );
}
