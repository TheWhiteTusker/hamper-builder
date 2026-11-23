import { useRef, useState, useTransition } from "react";
import { colorCode, deriveCategoryCode, formatProductCode, parseProductCode } from "@/lib/product-code";
import type { Category } from "@/lib/types";
import { getNextSerialAction } from "../../cost-calculator/serial-actions";

/**
 * The product code, built from the choices rather than typed:
 * CATEGORY/SERIAL/COLOUR (LC/0001/WL). Choosing a category takes the next
 * free serial in it; the colour suffix follows the first selected colour.
 */
export function useAutoCode(initialCode: string, categories: Category[], initialColors: string[]) {
  const [code, setCode] = useState(initialCode);
  const [fetching, startTransition] = useTransition();
  // The serial comes back from the server; by then the colours may have changed.
  const colorsRef = useRef(initialColors);

  const suffix = (colors: string[]) => (colors[0] ? colorCode(colors[0]) : parseProductCode(code).colorCode || "WL");

  function categoryChanged(categoryId: string) {
    const cat = categories.find((c) => c.id === categoryId);
    if (!cat) return; // "No category" leaves the code as it is
    const catCode = cat.code || deriveCategoryCode(cat.name);
    const current = parseProductCode(code);
    // Same category as the code already has: keep its serial.
    if (current.categoryCode === catCode && Number(current.serial) > 0) {
      return setCode(formatProductCode(catCode, current.serial, suffix(colorsRef.current)));
    }
    startTransition(async () => {
      const { serial } = await getNextSerialAction(catCode);
      setCode(formatProductCode(catCode, serial || "0001", suffix(colorsRef.current)));
    });
  }

  function colorsChanged(colors: string[]) {
    colorsRef.current = colors;
    const current = parseProductCode(code);
    // Nothing to put a suffix on until a category has given the code a prefix.
    if (!colors[0] || !current.categoryCode) return;
    setCode(formatProductCode(current.categoryCode, current.serial, colorCode(colors[0])));
  }

  return { code, setCode, fetching, categoryChanged, colorsChanged };
}
