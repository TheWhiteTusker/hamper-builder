"use client";

import Link from "@/components/link";
import { usePathname } from "next/navigation";
import { costingHref, historyHref } from "./href";

const tabClass = (active: boolean) =>
  `rounded-md px-3 py-1.5 text-xs font-semibold transition-all ${
    active
      ? "bg-white text-(--color-brand-dark) shadow-sm"
      : "text-(--color-muted) hover:text-(--color-ink)"
  }`;

/** The product open in the calculator or its history, from the URL; null on a blank calculator or Rates. */
function productCode(pathname: string): string | null {
  const rest = pathname.replace(/^\/cost-calculator\/?/, "").replace(/^history\/?/, "");
  if (!rest || rest.startsWith("master")) return null;
  return rest.split("/").map(decodeURIComponent).join("/");
}

export function CostTabs({ admin }: { admin: boolean }) {
  const pathname = usePathname();
  const master = pathname.startsWith("/cost-calculator/master");
  const history = pathname.startsWith("/cost-calculator/history");
  const code = productCode(pathname);
  return (
    <div className="flex items-center gap-1 rounded-lg bg-slate-100 p-1">
      {/* On a product's costing the tabs stay on that product. */}
      <Link
        href={master ? "/products/new" : code ? costingHref(code) : pathname}
        className={tabClass(!master && !history)}
      >
        {master ? "New product" : "Costing"}
      </Link>
      {/* This product's changes only. History is admin-only (0026_product_history.sql). */}
      {admin && code && (
        <Link href={historyHref(code)} className={tabClass(history)}>
          History
        </Link>
      )}
      <Link href="/cost-calculator/master" className={tabClass(master)}>
        Rates
      </Link>
    </div>
  );
}
