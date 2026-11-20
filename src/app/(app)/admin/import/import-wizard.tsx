"use client";

import { useState, useTransition } from "react";
import { runImport, type ImportReport } from "./actions";
import type { Grid, ImportInput } from "@/lib/import";
import { ImportReportView } from "./import-report";
import { LoadStep } from "./load-step";
import { SHEETS, parseCsv, type SheetKey } from "./sheets";

export function ImportWizard() {
  const [grids, setGrids] = useState<Partial<Record<SheetKey, Grid>>>({});
  const [fileNames, setFileNames] = useState<Partial<Record<SheetKey, string>>>({});
  const [report, setReport] = useState<ImportReport | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [pending, startTransition] = useTransition();
  const [parseError, setParseError] = useState<string | null>(null);

  /**
   * One .xlsx holding all six tabs. SheetJS is imported on demand so its
   * ~1MB does not sit in the page bundle for people using CSVs.
   */
  async function pickWorkbook(file: File | null) {
    setReport(null);
    setConfirmed(false);
    if (!file) return;

    try {
      const XLSX = await import("xlsx");
      const wb = XLSX.read(await file.arrayBuffer());

      const nextGrids: Partial<Record<SheetKey, Grid>> = {};
      const nextNames: Partial<Record<SheetKey, string>> = {};
      const missing: string[] = [];

      for (const sheet of SHEETS) {
        const ws = wb.Sheets[sheet.label];
        if (!ws) {
          missing.push(sheet.label);
          continue;
        }
        // raw: false so dates and percentages arrive as the text the sheet
        // shows, which is what the CSV path gives the mapper too.
        const grid = XLSX.utils.sheet_to_json<string[]>(ws, {
          header: 1,
          raw: false,
          defval: "",
          blankrows: false,
        });
        nextGrids[sheet.key] = grid;
        nextNames[sheet.key] = `${sheet.label} · ${Math.max(grid.length - 1, 0)} rows`;
      }

      setGrids(nextGrids);
      setFileNames(nextNames);
      setParseError(
        missing.length ? `${file.name} has no tab named: ${missing.join(", ")}` : null,
      );
    } catch {
      setParseError(`Could not read ${file.name}. Check it is a .xlsx workbook.`);
    }
  }

  async function pick(key: SheetKey, file: File | null) {
    setReport(null);
    setConfirmed(false);

    if (!file) {
      setGrids((g) => ({ ...g, [key]: undefined }));
      setFileNames((n) => ({ ...n, [key]: undefined }));
      return;
    }

    try {
      const grid = await parseCsv(file);
      setGrids((g) => ({ ...g, [key]: grid }));
      setFileNames((n) => ({ ...n, [key]: `${file.name} · ${Math.max(grid.length - 1, 0)} rows` }));
      setParseError(null);
    } catch {
      setParseError(`Could not read ${file.name}. Export it again as CSV.`);
    }
  }

  /** Several CSVs in one go, matched to tabs by file name (e.g. "Hamper Details.csv"). */
  async function pickMany(files: File[]) {
    const norm = (v: string) => v.toLowerCase().replace(/[^a-z]/g, "");
    const unmatched: string[] = [];
    for (const file of files) {
      const name = norm(file.name.replace(/\.csv$/i, ""));
      const sheet = SHEETS.find((s) => name.includes(norm(s.label)) || name.includes(norm(s.key)));
      if (sheet) await pick(sheet.key, file);
      else unmatched.push(file.name);
    }
    if (unmatched.length) {
      setParseError(`Couldn't tell which tab these are: ${unmatched.join(", ")}. Name each file after its tab, or use the pickers below.`);
    }
  }

  function run(apply: boolean) {
    startTransition(async () => {
      const result = await runImport(grids as ImportInput, apply);
      setReport(result);
      if (apply) setConfirmed(false);
    });
  }

  const anyLoaded = Object.values(grids).some(Boolean);

  return (
    <div className="space-y-4">      <LoadStep
        fileNames={fileNames}
        parseError={parseError}
        onWorkbook={pickWorkbook}
        onMany={pickMany}
        onPick={pick}
      />

      <section className="card p-4">
        <h2 className="text-sm font-semibold">2. Check what will be imported</h2>
        <p className="mt-1 text-sm text-(--color-muted)">
          A dry run reads the files and reports what it found. Nothing is written yet.
        </p>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <button
            type="button"
            className="btn-secondary"
            onClick={() => run(false)}
            disabled={!anyLoaded || pending}
          >
            {pending ? "Checking…" : "Dry run"}
          </button>
        </div>

        {report && <ImportReportView report={report} />}
      </section>

      <section className="card p-4">
        <h2 className="text-sm font-semibold">3. Import</h2>
        <p className="mt-1 text-sm text-(--color-muted)">
          Records are matched on product code, hamper code and document number, so running
          this twice updates rather than duplicates. Existing hamper and quotation lines are
          replaced by what is in the files.
        </p>

        <label className="mt-3 flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
            className="mt-0.5"
          />
          I have run the dry run above and the numbers look right.
        </label>

        <button
          type="button"
          className="btn-primary mt-3"
          onClick={() => run(true)}
          disabled={!anyLoaded || !confirmed || pending}
        >
          {pending ? "Importing…" : "Import into the database"}
        </button>
      </section>
    </div>
  );
}
