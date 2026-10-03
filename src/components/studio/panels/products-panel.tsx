"use client";

import { useMemo, useState } from "react";
import { ArrowLeft, Package, Search } from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { DRAG_MIME, type Editor, type PickerProduct } from "../editor";
import { cx, fieldCls, toolBtn } from "../studio-ui";
import { thumbProps } from "@/lib/product-images";

function setDragImage(e: React.DragEvent, product: PickerProduct, url: string) {
  e.dataTransfer.setData(DRAG_MIME, JSON.stringify({ product, url }));
  e.dataTransfer.effectAllowed = "copy";
}

export function ProductsPanel({
  ed,
  products,
  hamperProductIds,
}: {
  ed: Editor;
  products: PickerProduct[];
  hamperProductIds: string[];
}) {
  const [scope, setScope] = useState<"hamper" | "all">(hamperProductIds.length ? "hamper" : "all");
  const [q, setQ] = useState("");
  const [product, setProduct] = useState<PickerProduct | null>(null);
  const [images, setImages] = useState<string[] | null>(null);

  const list = useMemo(() => {
    const s = q.trim().toLowerCase();
    const inHamper = new Set(hamperProductIds);
    return products
      .filter((p) => scope === "all" || inHamper.has(p.id))
      .filter((p) => !s || p.name.toLowerCase().includes(s) || p.code.toLowerCase().includes(s))
      .slice(0, 120);
  }, [q, scope, products, hamperProductIds]);

  async function open(p: PickerProduct) {
    setProduct(p);
    setImages(null);
    const { data } = await createClient()
      .from("product_images")
      .select("url")
      .eq("product_id", p.id)
      .is("deleted_at", null)
      .order("is_primary", { ascending: false })
      .order("sort_order")
      .returns<{ url: string }[]>();
    const urls = (data ?? []).map((r) => r.url);
    setImages(urls.length ? urls : p.image_url ? [p.image_url] : []);
  }

  return (
    <div className="space-y-3">
      {ed.swapId && (
        <div className="flex items-center justify-between rounded-md bg-[var(--st-accent-soft)] px-3 py-2 text-[12px]">
          Pick an image to replace the selected one
          <button type="button" className="underline" onClick={() => ed.startSwap(null)}>
            Cancel
          </button>
        </div>
      )}

      {product ? (
        <>
          <button type="button" className={cx(toolBtn, "-ml-2")} onClick={() => setProduct(null)}>
            <ArrowLeft className="h-4 w-4" /> {product.name}
          </button>
          {images === null ? (
            <p className="text-[var(--st-muted)]">Loading images…</p>
          ) : images.length === 0 ? (
            <p className="text-[var(--st-muted)]">No images yet. Add some on the product page.</p>
          ) : (
            <>
              <p className="text-[12px] text-[var(--st-muted)]">Click to add, or drag onto the page.</p>
              <div className="grid grid-cols-2 gap-2">
                {images.map((url) => (
                  <button
                    key={url}
                    type="button"
                    draggable
                    onDragStart={(e) => setDragImage(e, product, url)}
                    onClick={() => ed.placeImage(product, url)}
                    className="checkerboard aspect-square overflow-hidden rounded-md border border-[var(--st-line)] hover:border-[var(--st-accent)]"
                  >
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img {...thumbProps(url)} alt="" className="h-full w-full object-contain" draggable={false} />
                  </button>
                ))}
              </div>
            </>
          )}
        </>
      ) : (
        <>
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-2 h-4 w-4 text-[var(--st-muted)]" />
            <input
              className={cx(fieldCls, "w-full pl-8")}
              placeholder="Search products"
              value={q}
              onChange={(e) => setQ(e.target.value)}
            />
          </div>
          <div className="flex gap-1.5">
            {(
              [
                ["hamper", `In this hamper (${hamperProductIds.length})`],
                ["all", "All products"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setScope(id)}
                className={cx(
                  "rounded-full border px-3 py-1 text-[12px]",
                  scope === id
                    ? "border-[var(--st-accent)] bg-[var(--st-accent-soft)] text-[var(--st-accent-strong)]"
                    : "border-[var(--st-line)] text-[var(--st-muted)] hover:text-[var(--st-text)]",
                )}
              >
                {label}
              </button>
            ))}
          </div>
          {list.length === 0 ? (
            <p className="text-[var(--st-muted)]">No products found.</p>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              {list.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  draggable={!!p.image_url}
                  onDragStart={(e) => p.image_url && setDragImage(e, p, p.image_url)}
                  onClick={() => open(p)}
                  className="group text-left"
                  title={`${p.name} (${p.code})`}
                >
                  <div className="checkerboard aspect-square overflow-hidden rounded-md border border-[var(--st-line)] group-hover:border-[var(--st-accent)]">
                    {p.image_url ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img {...thumbProps(p.image_url)} alt="" className="h-full w-full object-contain" draggable={false} />
                    ) : (
                      <div className="flex h-full items-center justify-center text-[var(--st-muted)]">
                        <Package className="h-6 w-6" />
                      </div>
                    )}
                  </div>
                  <div className="mt-1 line-clamp-2 text-[12px] leading-tight">{p.name}</div>
                </button>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
