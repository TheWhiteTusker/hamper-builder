/**
 * What a quote line says the client is getting.
 *
 * Detail Mode and Packaging Treatment were captured on every quote line in the
 * spreadsheet but never used - there was no printed document to use them in.
 *
 * Both are free-text pick-lists, so they are matched loosely: anything with
 * "hide" hides the contents, "summary" prints just a count, anything else
 * lists them. Packaging works the same way - "absorb" folds packaging items
 * into the hamper, anything else lists them and names the treatment.
 *
 * Rendered by both the print sheet and the quote builder, so what the client
 * will see is exactly what the person building the quote sees.
 */
export const showsContents = (mode: string | null) => !/hide/i.test(mode ?? "");
export const summaryOnly = (mode: string | null) => /summary|count/i.test(mode ?? "");
export const absorbsPackaging = (treatment: string | null) => /absorb/i.test(treatment ?? "");

/** The slice of a hamper item this needs; both HamperItem and the browser copy fit. */
export type ContentItem = {
  id: string;
  product_name: string;
  category_name: string | null;
  qty: number;
};

/** Packaging is whatever Settings marks as not counting toward No. of Items. */
export const packagingLookup = (packagingCategories: string[]) => {
  const set = new Set(packagingCategories.map((c) => c.trim().toLowerCase()));
  return (category: string | null) => set.has((category ?? "").trim().toLowerCase());
};

export function HamperContents({
  items,
  detailMode,
  packagingTreatment,
  packagingCategories,
  className = "",
}: {
  items: ContentItem[];
  detailMode: string | null;
  packagingTreatment: string | null;
  packagingCategories: string[];
  className?: string;
}) {
  const isPackaging = packagingLookup(packagingCategories);
  const absorb = absorbsPackaging(packagingTreatment);
  const visible = absorb ? items.filter((i) => !isPackaging(i.category_name)) : items;

  return (
    <div className={className}>
      {showsContents(detailMode) &&
        visible.length > 0 &&
        (summaryOnly(detailMode) ? (
          <div className="mt-0.5 text-(--color-muted)">{visible.length} items</div>
        ) : (
          <ul className="mt-1 space-y-0.5 text-(--color-muted)">
            {visible.map((item) => (
              <li key={item.id}>
                {item.product_name}
                {Number(item.qty) !== 1 && ` × ${item.qty}`}
              </li>
            ))}
          </ul>
        ))}

      {!absorb && packagingTreatment && (
        <div className="mt-1 text-xs text-(--color-muted)">
          Packaging: {packagingTreatment}
        </div>
      )}
    </div>
  );
}
