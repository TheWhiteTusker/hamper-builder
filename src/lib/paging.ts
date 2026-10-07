/** Rows on first load of the infinite-scroll lists. */
export const PAGE_SIZE = 20;

/** Rows added each time the list is scrolled to the end. */
export const STEP_SIZE = 10;

/** ?limit= from the URL, never less than one page. */
export const pageLimit = (raw: string) =>
  Math.max(PAGE_SIZE, Number.parseInt(raw, 10) || PAGE_SIZE);
