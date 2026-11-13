"use client";

import { useState, useTransition, type ReactNode } from "react";
import Link from "@/components/link";
import type { Grid } from "@/lib/import";
import type { ImportReport } from "./actions";
import { ImportReportView } from "./import-report";

/**
 * Download → fill in Excel → upload → save. The upload is checked first; nothing
 * is written until Save. Sheets are built and read in the browser (SheetJS on
 * demand) to keep the work off the Worker's CPU budget.
 */
export function SheetWizard({
  tips,
  downloadLabel,
  fileName,
  saveLabel,
  saveText,
  doneHref,
  doneLabel,
  build,
  check,
}: {
  tips: string[];
  downloadLabel: string;
  fileName: string;
  saveLabel: string;
  /** The Save button's wording once the check has counted what will change. */
  saveText?: (counts: Record<string, number>) => string;
  doneHref: string;
  doneLabel: string;
  build: () => Promise<Uint8Array>;
  check: (grid: Grid, apply: boolean) => Promise<ImportReport>;
}) {
  const [downloaded, setDownloaded] = useState(false);
  const [grid, setGrid] = useState<Grid | null>(null);
  const [loaded, setLoaded] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [dragging, setDragging] = useState(false);
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
      setDownloaded(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Couldn't prepare the file. Try again.");
    } finally {
      setDownloading(false);
    }
  }

  function run(rows: Grid, apply: boolean) {
    startTransition(async () => setReport(await check(rows, apply)));
  }

  async function pick(file: File | null | undefined) {
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
      setError(`Couldn't read ${file.name}. Use the .xlsx file from step 1.`);
    }
  }

  const counts = report?.counts ?? {};
  const anything = Object.values(counts).some(Boolean);
  const saved = !!report?.applied && report.ok;

  return (
    <div className="space-y-3">
      <Step n={1} title="Download" done={downloaded || !!grid}>
        <ul className="list-disc space-y-0.5 pl-5 text-sm text-(--color-muted)">
          {tips.map((t) => (
            <li key={t}>{t}</li>
          ))}
          <li>Blue headings: type here. Grey: worked out for you.</li>
        </ul>
        <button type="button" className="btn-secondary mt-3" onClick={download} disabled={downloading}>
          {downloading ? "Preparing…" : downloadLabel}
        </button>
      </Step>

      <Step n={2} title="Upload" done={!!grid}>
        <label
          onDragOver={(e) => (e.preventDefault(), setDragging(true))}
          onDragLeave={() => setDragging(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDragging(false);
            pick(e.dataTransfer.files[0]);
          }}
          className={`flex cursor-pointer flex-col items-center gap-1 rounded-lg border-2 border-dashed px-4 py-6 text-center text-sm transition-colors ${
            dragging ? "border-(--color-brand) bg-(--color-brand-soft)" : "border-line hover:bg-paper"
          }`}
        >
          <span className="font-medium">{loaded || "Drop your Excel file here"}</span>
          <span className="text-xs text-(--color-muted)">{loaded ? "Click to choose a different file" : "or click to choose"}</span>
          <input
            type="file"
            accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            className="sr-only"
            onChange={(e) => pick(e.target.files?.[0])}
          />
        </label>
        {error && (
          <p role="alert" className="mt-3 text-sm text-red-700">
            {error}
          </p>
        )}
        {pending && !report && <p className="mt-3 text-sm text-(--color-muted)">Checking…</p>}
        {report && <ImportReportView report={report} />}
      </Step>

      <Step n={3} title="Save" done={saved}>
        <p className="text-sm text-(--color-muted)">Nothing changes until you press this.</p>
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            className="btn-primary"
            onClick={() => grid && run(grid, true)}
            disabled={!grid || pending || !report?.ok || report.applied || !anything}
          >
            {pending && report ? "Saving…" : report?.ok && !report.applied && anything && saveText ? saveText(counts) : saveLabel}
          </button>
          {saved && (
            <Link href={doneHref} className="btn-secondary">
              {doneLabel}
            </Link>
          )}
        </div>
      </Step>
    </div>
  );
}

function Step({ n, title, done, children }: { n: number; title: string; done: boolean; children: ReactNode }) {
  return (
    <section className="card p-4">
      <h2 className="flex items-center gap-2 text-sm font-semibold">
        <span
          className={`flex h-6 w-6 items-center justify-center rounded-full text-xs font-bold ${
            done ? "bg-emerald-600 text-white" : "bg-(--color-brand-soft) text-(--color-brand-dark)"
          }`}
        >
          {done ? "✓" : n}
        </span>
        {title}
      </h2>
      <div className="mt-3">{children}</div>
    </section>
  );
}
