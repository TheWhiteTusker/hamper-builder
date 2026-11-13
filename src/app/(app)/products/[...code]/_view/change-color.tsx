"use client";

import { useActionState, useState } from "react";
import { parseProductCode, type ProductColor } from "@/lib/product-code";
import { recolorProduct } from "../../actions";

/**
 * Corrects this one product's colour. The code's last part follows it, and so
 * do its cost sheet, hamper lines and photos. Other colours are not touched.
 */
export function ChangeColor({
  id,
  code,
  current,
  palette,
}: {
  id: string;
  code: string;
  /** This product's colour; empty when none was ever saved. */
  current: string;
  palette: ProductColor[];
}) {
  const [color, setColor] = useState(current);
  const [state, action, pending] = useActionState(recolorProduct, {});
  const { categoryCode, serial } = parseProductCode(code);
  const next = palette.find((c) => c.name === color);
  // Mirrors recolor_product: only the last part of the code changes.
  const nextCode = next && categoryCode ? `${categoryCode}/${serial}/${next.code}` : code;

  return (
    <form action={action} className="flex flex-wrap items-center gap-2">
      <input type="hidden" name="id" value={id} />
      <label htmlFor="change-color" className="text-xs font-semibold text-(--color-ink)">
        Colour
      </label>
      <select
        id="change-color"
        name="color"
        value={color}
        onChange={(e) => setColor(e.target.value)}
        className="select py-1 text-xs"
      >
        {!current && <option value="">Not set</option>}
        {palette.map((c) => (
          <option key={c.name} value={c.name}>
            {c.name} ({c.code})
          </option>
        ))}
      </select>
      {color !== current && next && (
        <button type="submit" disabled={pending} className="btn-secondary py-1 text-xs">
          {pending ? "Changing…" : nextCode !== code ? `Change to ${nextCode}` : "Save colour"}
        </button>
      )}
      {state.error && (
        <span role="alert" className="text-xs text-red-700">
          {state.error}
        </span>
      )}
    </form>
  );
}
