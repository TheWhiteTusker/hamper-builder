"use client";

import { useState, useTransition } from "react";
import Link from "@/components/link";
import type { Grid } from "@/lib/import";
import type { ImportReport } from "./actions";
import { ImportReportView } from "./import-report";

/**
 * Download an Excel sheet → fill it in → upload it back: checked first, saved
 * on a second click. Sheets are built and read in the browser (SheetJS loaded
 * on demand) to keep the work off the Worker's CPU budget.
 */
export function SheetWizard({
  intro,
  downloadLabel,
  fileName,
  saveLabel,
  doneHref,
  doneLabel,
  build,
  check,
}: {
  intro: string;
  downloadLabel: string;
  fileName: string;
  saveLabel: string;
  /** Offered once saved, e.g. back to the list. */
  doneHref: string;
  doneLabel: string;
  build: () => Promise<Uint8Array>;
  check: (grid: Grid, apply: boolean) => Promise<ImportReport>;
}) {
  const [grid, setGrid] = useState<Grid | null>(null);
  const [loaded, setLoaded] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [pending, startTransition] = useTransition();

  async function download() {
    setDownloading(true);
    setError(null);
    try {
      const blob = new Blob([(await build()) as BlobPart], {
        type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
      });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(a.href);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not prepare the Excel file. Try again.");
    } finally {
      setDownloading(false);
    }
  }

  function run(rows: Grid, apply: boolean) {
    startTransition(async () => setReport(await check(rows, apply)));
  }

  async function pick(file: File | null) {
    setGrid(null);
    setReport(null);
    setError(null);
    if (!file) return;
    try {
      const XLSX = await import("xlsx");
      const wb = XLSX.read(await file.arrayBuffer());
      const ws = wb.Sheets[wb.SheetNames[0]]; // the downloads have one tab
      // raw: false so percentages arrive as "35.00%", which the mappers expect.
      const rows = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1, raw: false, defval: "", blankrows: false });
      setGrid(rows);
      setLoaded(file.name);
      run(rows, false);
    } catch {
      setError(`Could not read ${file.name}. Check it is the .xlsx downloaded above.`);
    }
  }

  const anything = Object.values(report?.counts ?? {}).some(Boolean);

  return (
    <div className="space-y-4">
      <section className="card p-4">
        <h2 className="text-sm font-semibold">1. Download the Excel sheet</h2>
        <p className="mt-1 text-sm text-[var(--color-muted)]">{intro}</p>
        <p className="mt-1 text-sm text-[var(--color-muted)]">
          Blue headings are for typing in; grey headings are worked out by a formula. Keep the
          columns in the same order.
        </p>
        <button type="button" className="btn-secondary mt-3" onClick={download} disabled={downloading}>
          {downloading ? "Preparing…" : downloadLabel}
        </button>
      </section>

      <section className="card p-4">
        <h2 className="text-sm font-semibold">2. Upload the filled sheet</h2>
        <p className="mt-1 text-sm text-[var(--color-muted)]">
          It is checked first. Nothing is saved until you press the button below.
        </p>
        <input
          type="file"
          accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
          className="mt-3 w-full text-sm"
          onChange={(e) => pick(e.target.files?.[0] ?? null)}
        />
        {loaded && <p className="mt-1 text-xs text-green-800">{loaded}</p>}
        {error && (
          <p role="alert" className="mt-3 text-sm text-red-700">
            {error}
          </p>
        )}
        {pending && !report && <p className="mt-3 text-sm text-[var(--color-muted)]">Checking…</p>}
        {report && <ImportReportView report={report} />}
      </section>

      <section className="card p-4">
        <h2 className="text-sm font-semibold">3. Save</h2>
        <button
          type="button"
          className="btn-primary mt-3"
          onClick={() => grid && run(grid, true)}
          disabled={!grid || pending || !report?.ok || report.applied || !anything}
        >
          {pending ? "Saving…" : saveLabel}
        </button>
        {report?.applied && report.ok && (
          <Link href={doneHref} className="btn-secondary mt-3 ml-2">
            {doneLabel}
          </Link>
        )}
      </section>
    </div>
  );
}
