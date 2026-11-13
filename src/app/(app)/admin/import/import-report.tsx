"use client";

import Link from "@/components/link";
import type { ImportReport } from "./actions";

/** What a dry run (or the real import) found: counts, warnings, and "done". */
export function ImportReportView({ report }: { report: ImportReport }) {
  return (
    <div className="mt-4">
      {report.error ? (
        <p role="alert" className="text-sm text-red-700">
          {report.error}
        </p>
      ) : (
        <>
          <div className="grid gap-2 sm:grid-cols-3">
            {Object.entries(report.counts ?? {}).map(([key, value]) => (
              <div
                key={key}
                className="rounded-md border border-[var(--color-line)] px-3 py-2"
              >
                <div className="label">{labelFor(key)}</div>
                <div className="text-lg font-semibold tabular-nums">{value}</div>
              </div>
            ))}
          </div>

          {!!report.warnings?.length && (
            <ul className="mt-3 space-y-1.5 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
              {report.warnings.map((w) => (
                <li key={w}>{w}</li>
              ))}
            </ul>
          )}

          {report.applied && (
            <p className="mt-3 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">
              {report.message ??
                "Import complete. Document numbering now continues from the highest imported code."}
            </p>
          )}

          {report.applied && !!report.added?.length && (
            <div className="mt-3 rounded-md border border-[var(--color-line)] p-3">
              <div className="label">Newly added ({report.added.length})</div>
              <ul className="mt-1.5 max-h-72 space-y-1 overflow-y-auto text-sm">
                {report.added.map((a) => (
                  <li key={a.label}>
                    {a.href ? (
                      <Link href={a.href} className="text-(--color-brand) underline">
                        {a.label}
                      </Link>
                    ) : (
                      a.label
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function labelFor(key: string) {
  const labels: Record<string, string> = {
    categories: "Categories",
    products: "Products",
    hampers: "Hampers",
    hamperItems: "Hamper lines",
    quotes: "Quotations",
    quoteItems: "Quotation lines",
    newCategories: "New categories",
    newSubcategories: "New subcategories",
    newVarieties: "New varieties",
    updatedVarieties: "Varieties changed",
  };
  return labels[key] ?? key;
}
