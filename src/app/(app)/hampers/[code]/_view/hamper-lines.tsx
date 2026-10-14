import Image from "next/image";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatMoney, formatPct } from "@/lib/pricing";
import { thumbProps } from "@/lib/product-images";
import type { HamperItem } from "@/lib/types";

/** A saved line plus its product's photo, if the product still exists. */
export type PreviewItem = HamperItem & { products: { image_url: string | null } | null };

/** The hamper's products as saved, in the builder's columns, read-only. */
export function HamperLines({ items }: { items: PreviewItem[] }) {
  return (
    <section className="card overflow-hidden">
      <div className="border-b border-[var(--color-line)] px-4 py-2.5">
        <h2 className="text-sm font-semibold">
          Products
          <span className="ml-2 font-normal text-[var(--color-muted)]">
            {items.length} line{items.length === 1 ? "" : "s"}
          </span>
        </h2>
      </div>

      <div className="overflow-x-auto">
        <Table className="min-w-[960px]">
          <TableHeader>
            <TableRow>
              <TableHead>Product</TableHead>
              <TableHead>Code</TableHead>
              <TableHead className="num">Qty</TableHead>
              <TableHead className="num">Unit cost</TableHead>
              <TableHead className="num">Total cost</TableHead>
              <TableHead className="num">Margin %</TableHead>
              <TableHead className="num">Unit price</TableHead>
              <TableHead className="num">Total price</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item) => (
              <TableRow key={item.id}>
                <TableCell>
                  <div className="flex items-center gap-2">
                    {item.products?.image_url && (
                      <div className="relative h-8 w-8 shrink-0 overflow-hidden rounded-md border border-slate-200 bg-slate-100">
                        <Image {...thumbProps(item.products.image_url)} alt="" fill sizes="32px" className="object-cover" unoptimized />
                      </div>
                    )}
                    <div>
                      <div className="font-medium">{item.product_name}</div>
                      {item.category_name && <div className="text-xs text-(--color-muted)">{item.category_name}</div>}
                    </div>
                  </div>
                </TableCell>
                <TableCell className="font-mono text-xs text-(--color-muted)">{item.product_code || "—"}</TableCell>
                <TableCell className="num">{item.qty}</TableCell>
                <TableCell className="num">{formatMoney(item.unit_cp)}</TableCell>
                <TableCell className="num text-(--color-muted)">{formatMoney(item.total_cp)}</TableCell>
                <TableCell className="num">{formatPct(item.target_margin)}</TableCell>
                <TableCell className="num">{formatMoney(item.unit_sp)}</TableCell>
                <TableCell className="num font-medium">{formatMoney(item.total_sp)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}
