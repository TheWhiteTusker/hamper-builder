/** Sheet grids and the cell readers the importer uses. */

export type Grid = string[][];

export type ImportInput = {
  settings?: Grid;
  products?: Grid;
  hamperSummary?: Grid;
  hamperDetails?: Grid;
  quotationRegister?: Grid;
  quoteDetails?: Grid;
};

export const cell = (row: string[] | undefined, i: number) => String(row?.[i] ?? "").trim();
export const isYes = (v: string) => /^(yes|y|true|1|active)$/i.test(v.trim());
const filled = (r: string[]) => r.some((c) => String(c).trim());
export const body = (grid?: Grid) => (grid ?? []).slice(1).filter(filled);
/** Like body(), with each row's number in the sheet; the grid must keep blank rows (blankrows: true). */
export const numbered = (grid?: Grid) =>
  (grid ?? []).flatMap((row, i) => (i > 0 && filled(row) ? [{ row, n: i + 1 }] : []));

/**
 * Sheets export dates in whatever the locale was. ISO is tried first, then
 * D/M/Y, which is the Indian convention this business uses. Anything else is
 * reported rather than guessed at.
 */
export function parseDate(value: string): string | null {
  const raw = value.trim();
  if (!raw) return null;

  if (/^\d{4}-\d{2}-\d{2}/.test(raw)) return raw.slice(0, 10);

  const dmy = raw.match(/^(\d{1,2})[/-](\d{1,2})[/-](\d{4})/);
  if (dmy) {
    const [, d, m, y] = dmy;
    return `${y}-${m.padStart(2, "0")}-${d.padStart(2, "0")}`;
  }

  const parsed = new Date(raw);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString().slice(0, 10);
}
