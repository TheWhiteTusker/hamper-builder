/** Rows per step on the infinite-scroll lists. */
export const PAGE_SIZE = 20;

/** ?limit= from the URL, never less than one page. */
export const pageLimit = (raw: string) =>
  Math.max(PAGE_SIZE, Number.parseInt(raw, 10) || PAGE_SIZE);
