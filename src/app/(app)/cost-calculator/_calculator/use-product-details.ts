import { useState } from "react";
import { useRouter } from "next/navigation";
import { parseProductCode } from "@/lib/product-code";
import type { Category, CostStageWithHierarchy, Product, ProductCostSheet } from "@/lib/types";
import { costingHref } from "../href";
import { useAutoCode } from "../../products/_form/use-auto-code";
import { matchOrigin } from "./lines";

export type ProductDetails = ReturnType<typeof useProductDetails>;

/** Code, name, category, origin, colours and the other product-level fields. */
export function useProductDetails({
  stages,
  categories,
  products,
  initialProduct,
  initialSheet,
}: {
  stages: CostStageWithHierarchy[];
  categories: Category[];
  products: Pick<Product, "id" | "code" | "name">[];
  initialProduct?: Product | null;
  initialSheet?: ProductCostSheet | null;
}) {
  const router = useRouter();
  const [selectedProductId, setSelectedProductId] = useState(initialProduct?.id ?? "");
  const [name, setName] = useState(initialProduct?.name ?? initialSheet?.product_name ?? "");
  const [categoryId, setCategoryId] = useState(initialProduct?.category_id ?? "");
  const [source, setSource] = useState<string>(() => matchOrigin(initialProduct?.source ?? "In-house"));
  // Stage codes whose line table is hidden. Outsource collapses manufacturing stages.
  const [collapsedStages, setCollapsedStages] = useState<Set<string>>(() =>
    (initialProduct?.source ?? "").toLowerCase() === "outsource"
      ? new Set(stages.filter((s) => s.code !== "bought_out").map((s) => s.code))
      : new Set(),
  );
  const [selectedColors, setSelectedColors] = useState<string[]>(initialProduct?.colors ?? ["Walnut"]);
  // The code follows the category and colour choices (see useAutoCode).
  const auto = useAutoCode(initialProduct?.code ?? initialSheet?.product_code ?? "", categories, selectedColors);
  const { code, setCode } = auto;
  const [isActive, setIsActive] = useState(initialProduct ? initialProduct.is_active : true);
  const [notes, setNotes] = useState(initialSheet?.notes ?? "");
  const parsedCode = parseProductCode(code);

  // Each product's costing is its own page; a blank one is /products/new.
  function selectProduct(prodId: string) {
    setSelectedProductId(prodId);
    const prod = products.find((p) => p.id === prodId);
    router.push(prod ? costingHref(prod.code) : "/products/new");
  }

  // Product Origin decides whether the stages start collapsed.
  function changeOrigin(origin: string) {
    setSource(origin);
    setCollapsedStages(
      origin === "Outsource"
        ? new Set(stages.filter((s) => s.code !== "bought_out").map((s) => s.code))
        : new Set(),
    );
  }

  function toggleStage(stageCode: string) {
    setCollapsedStages((prev) => {
      const next = new Set(prev);
      if (!next.delete(stageCode)) next.add(stageCode);
      return next;
    });
  }

  function changeCategory(newCatId: string) {
    setCategoryId(newCatId);
    auto.categoryChanged(newCatId);
  }

  // A product can come in several finishes; the code suffix follows the first.
  function toggleColor(colName: string) {
    const next = selectedColors.includes(colName)
      ? selectedColors.filter((c) => c !== colName)
      : [...selectedColors, colName];
    setSelectedColors(next);
    auto.colorsChanged(next);
  }

  return {
    selectedProductId, setSelectedProductId, code, setCode, name, setName, categoryId, source,
    collapsedStages, selectedColors, isActive, setIsActive, notes, setNotes, parsedCode,
    selectProduct, changeOrigin, toggleStage, changeCategory, toggleColor, fetchingCode: auto.fetching,
  };
}
