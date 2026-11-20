"use client";

import { SHEETS, type SheetKey } from "./sheets";

/** Step 1: a whole workbook, several CSVs at once, or one CSV per tab. */
export function LoadStep({
  fileNames,
  parseError,
  onWorkbook,
  onMany,
  onPick,
}: {
  fileNames: Partial<Record<SheetKey, string>>;
  parseError: string | null;
  onWorkbook: (file: File | null) => void;
  onMany: (files: File[]) => void;
  onPick: (key: SheetKey, file: File | null) => void;
}) {
  return (
    <section className="card p-4">
      <h2 className="text-sm font-semibold">1. Load the spreadsheet</h2>
      <p className="mt-1 text-sm text-(--color-muted)">
        Easiest: File → Download → Microsoft Excel (.xlsx) once, and drop the whole
        workbook here. Columns are read by position, so leave the layout exactly as it is —
        renamed headers are fine, reordered columns are not.
      </p>

      <div className="mt-3 rounded-md border border-(--color-line) p-3">
        <label className="label" htmlFor="workbook">
          Whole workbook (.xlsx)
        </label>
        <p className="mt-0.5 text-xs text-(--color-muted)">
          Splits into the six tabs below by name
        </p>
        <input
          id="workbook"
          type="file"
          accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
          className="mt-2 w-full text-sm"
          onChange={(e) => onWorkbook(e.target.files?.[0] ?? null)}
        />
      </div>

      <div className="mt-4 rounded-md border border-(--color-line) p-3">
        <label className="label" htmlFor="csv-many">
          Several CSV files at once
        </label>
        <p className="mt-0.5 text-xs text-(--color-muted)">
          Select them together; each is matched to its tab by file name, e.g. “Product Master.csv”
        </p>
        <input
          id="csv-many"
          type="file"
          accept=".csv,text/csv"
          multiple
          className="mt-2 w-full text-sm"
          onChange={(e) => onMany(Array.from(e.target.files ?? []))}
        />
      </div>

      <p className="mt-4 text-sm text-(--color-muted)">
        Or load tabs one at a time, as CSV. These override anything the workbook filled in.
      </p>

      <div className="mt-3 grid gap-3 sm:grid-cols-2">
        {SHEETS.map((sheet) => (
          <div key={sheet.key} className="rounded-md border border-(--color-line) p-3">
            <label className="label" htmlFor={sheet.key}>
              {sheet.label}
            </label>
            <p className="mt-0.5 text-xs text-(--color-muted)">{sheet.hint}</p>
            <input
              id={sheet.key}
              type="file"
              accept=".csv,text/csv"
              className="mt-2 w-full text-sm"
              onChange={(e) => onPick(sheet.key, e.target.files?.[0] ?? null)}
            />
            {fileNames[sheet.key] && (
              <p className="mt-1 text-xs text-green-800">{fileNames[sheet.key]}</p>
            )}
          </div>
        ))}
      </div>

      {parseError && (
        <p role="alert" className="mt-3 text-sm text-red-700">
          {parseError}
        </p>
      )}
    </section>
  );
}
