import { useState } from "react";
import { parseProductCode } from "@/lib/product-code";
import type { Category, CostStageWithHierarchy, Product, ProductCostSheet } from "@/lib/types";
import { useAutoCode } from "../../products/_form/use-auto-code";
import { matchOrigin } from "./lines";

export type ProductDetails = ReturnType<typeof useProductDetails>;

/**
 * Code, name, category, origin, colours and the other product-level fields.
 * Every product is one colour. A new product can be saved in several colours
 * at once (one product each). An existing one keeps its colour and code; new
 * colours added while editing become new products on save.
 */
export function useProductDetails({
  stages,
  categories,
  initialProduct,
  initialSheet,
  ownColor = "",
  siblingColors = [],
}: {
  stages: CostStageWithHierarchy[];
  categories: Category[];
  initialProduct?: Product | null;
  initialSheet?: ProductCostSheet | null;
  /** The edited product's own colour. */
  ownColor?: string;
  /** Colours that already exist as other products of this one. */
  siblingColors?: string[];
}) {
  const editing = !!initialProduct;
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
  // Editing: this product's colour first (fixed), then any new ones added.
  const [selectedColors, setSelectedColors] = useState<string[]>(editing ? [ownColor].filter(Boolean) : ["Walnut"]);
  // The code follows the category and colour choices (see useAutoCode).
  const auto = useAutoCode(initialProduct?.code ?? initialSheet?.product_code ?? "", categories, selectedColors);
  const { code, setCode } = auto;
  const [isActive, setIsActive] = useState(initialProduct ? initialProduct.is_active : true);
  const [notes, setNotes] = useState(initialSheet?.notes ?? "");
  const parsedCode = parseProductCode(code);

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
    if (editing) return; // the category is part of the code, fixed once created
    setCategoryId(newCatId);
    auto.categoryChanged(newCatId);
  }

  // A product can come in several finishes; the code suffix follows the first.
  function toggleColor(colName: string) {
    if (editing && colName === ownColor) return; // a product's own colour never changes here
    const next = selectedColors.includes(colName)
      ? selectedColors.filter((c) => c !== colName)
      : [...selectedColors, colName];
    setSelectedColors(next);
    if (!editing) auto.colorsChanged(next); // an existing product's code never changes here
  }

  return {
    selectedProductId, setSelectedProductId, code, setCode, name, setName, categoryId, source,
    collapsedStages, selectedColors, isActive, setIsActive, notes, setNotes, parsedCode,
    changeOrigin, toggleStage, changeCategory, toggleColor, fetchingCode: auto.fetching,
    editing, ownColor, siblingColors,
  };
}
