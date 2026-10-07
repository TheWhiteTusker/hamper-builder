"use client";

import Link from "@/components/link";
import { usePathname } from "next/navigation";

const tabClass = (active: boolean) =>
  `rounded-md px-3 py-1.5 text-xs font-semibold transition-all ${
    active
      ? "bg-white text-[var(--color-brand-dark)] shadow-sm"
      : "text-[var(--color-muted)] hover:text-[var(--color-ink)]"
  }`;

export function CostTabs() {
  const pathname = usePathname();
  const master = pathname.startsWith("/cost-calculator/master");
  return (
    <div className="flex items-center gap-1 rounded-lg bg-slate-100 p-1">
      {/* On a product's costing the tab stays on that product. */}
      <Link href={master ? "/products/new" : pathname} className={tabClass(!master)}>
        {master ? "New Product" : "Costing Editor"}
      </Link>
      <Link href="/cost-calculator/master" className={tabClass(master)}>
        Rates & Hierarchy Master
      </Link>
    </div>
  );
}
