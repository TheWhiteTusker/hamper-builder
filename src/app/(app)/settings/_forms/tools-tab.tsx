import Link from "@/components/link";
import { UpdateAppButton } from "./update-app-button";

// Most-used first; the one-time import sits at the bottom of the tab.
const TOOLS = [
  { href: "/bin", title: "Recycle Bin", hint: "Restore deleted hampers, products and photos. Kept for 30 days." },
  { href: "/admin/refresh-prices", title: "Refresh prices", hint: "Update saved hampers with today's product prices." },
];

/** Settings → Tools: the bin, price refresh, app updates and the one-time import. */
export function ToolsTab({ desktop, version }: { desktop: boolean; version: string | null }) {
  return (
    <section className="card divide-y divide-(--color-line)">
      {TOOLS.map((t) => (
        <div key={t.href} className="flex flex-wrap items-center justify-between gap-3 p-4">
          <div>
            <h2 className="text-sm font-semibold">{t.title}</h2>
            <p className="mt-0.5 text-xs text-(--color-muted)">{t.hint}</p>
          </div>
          <Link href={t.href} className="btn-secondary">
            Open
          </Link>
        </div>
      ))}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4">
        <div>
          <h2 className="text-sm font-semibold">App</h2>
          {/* The app shows its own version; the website, the newest release. */}
          <p className="mt-0.5 text-xs text-(--color-muted)">
            Lattice Lane Hamper Builder
            {version && (
              <>
                {" "}· {desktop ? "Version" : "Latest release"}{" "}
                <span className="font-mono font-medium text-(--color-brand)">v{version}</span>
              </>
            )}
          </p>
        </div>
        <UpdateAppButton desktop={desktop} />
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 p-4">
        <div>
          <h2 className="text-sm font-semibold">
            Import old workbook <span className="badge ml-1 align-middle">One-time</span>
          </h2>
          <p className="mt-0.5 text-xs text-(--color-muted)">
            Moves products, hampers and quotes across from the old 6-tab spreadsheet.
          </p>
        </div>
        <Link href="/admin/import" className="btn-secondary">
          Open
        </Link>
      </div>
    </section>
  );
}
