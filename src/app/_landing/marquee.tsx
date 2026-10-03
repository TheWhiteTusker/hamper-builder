import type { DisplayProduct } from "./fallbacks";
import { thumbProps } from "@/lib/product-images";

/**
 * Ultra-smooth infinite downward scrolling column.
 * Two identical sub-containers with identical vertical padding (pb-3) ensure the
 * translate3d(0, -50%, 0) to translate3d(0, 0, 0) transition is mathematically seamless
 * with zero subpixel jitter or seam jumping.
 */
export function ScrollingProductColumn({
  items,
  durationSeconds,
  label,
}: {
  items: DisplayProduct[];
  durationSeconds: number;
  label: string;
}) {
  if (items.length === 0) return null;

  return (
    <div
      className="flex flex-col animate-marquee-down"
      style={{ animationDuration: `${durationSeconds}s` }}
      aria-label={label}
    >
      {/* Group 1 */}
      <div className="flex flex-col gap-3 pb-3">
        {items.map((item) => (
          <ProductCard key={`g1-${item.id}`} item={item} />
        ))}
      </div>

      {/* Group 2 (Identical clone for continuous seam-free loop) */}
      <div className="flex flex-col gap-3 pb-3" aria-hidden="true">
        {items.map((item) => (
          <ProductCard key={`g2-${item.id}`} item={item} />
        ))}
      </div>
    </div>
  );
}

function ProductCard({ item }: { item: DisplayProduct }) {
  return (
    <article className="shrink-0 w-full rounded-2xl bg-white p-2.5 border border-[var(--color-line)] shadow-xs transition-shadow duration-200 group cursor-default">
      <div className="aspect-[4/3] w-full rounded-xl overflow-hidden bg-[var(--color-paper)] relative">
        {/* eslint-disable-next-line @next/next/no-img-element -- remote catalogue and Unsplash photos */}
        <img
          {...thumbProps(item.url)}
          alt={item.name}
          decoding="async"
          loading="lazy"
          className="h-full w-full object-cover transition-transform duration-300 ease-out group-hover:scale-105"
        />
      </div>
      <div className="mt-2 text-left px-0.5">
        <p className="text-[10px] sm:text-[11px] font-semibold tracking-wider uppercase text-[var(--color-muted)] truncate">
          {item.category}
        </p>
        <h3 className="text-xs sm:text-sm font-medium text-[var(--color-ink)] truncate group-hover:text-[var(--color-brand)] transition-colors">
          {item.name}
        </h3>
      </div>
    </article>
  );
}
