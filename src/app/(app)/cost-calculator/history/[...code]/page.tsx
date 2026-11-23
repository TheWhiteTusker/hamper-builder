import { notFound } from "next/navigation";
import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import type { Category } from "@/lib/types";
import { ProductHistory } from "../../../products/_history/product-history";
import { productHref } from "../../../products/href";

/** The calculator's History tab: who changed this product, when, and what. Admins only. */
export default async function CostingHistoryPage({ params }: { params: Promise<{ code: string[] }> }) {
  const { code: parts } = await params;
  const code = decodeURIComponent(parts.join("/"));
  const { supabase } = await requireRole("admin");

  const [{ data: product }, { data: categories }] = await Promise.all([
    supabase.from("products").select("id, code, name").eq("code", code).maybeSingle(),
    supabase.from("categories").select("*").overrideTypes<Category[], { merge: false }>(),
  ]);
  if (!product) notFound();

  return (
    <div className="space-y-3">
      <p className="text-sm">
        <Link href={productHref(product.code)} className="font-mono font-semibold hover:underline">
          {product.code}
        </Link>{" "}
        <span className="text-(--color-muted)">{product.name}</span>
      </p>
      <ProductHistory productId={product.id} categories={categories ?? []} shown={50} />
    </div>
  );
}
