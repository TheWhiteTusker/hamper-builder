"use client";

import { useRef, useState, useTransition } from "react";
import {
  STANDARD_PRODUCT_COLORS,
  FALLBACK_COLOR_HEX,
  colorCode,
  type ProductColor,
} from "@/lib/product-code";
import { saveProductColors } from "./actions";

export function ColorsCard({ productColors }: { productColors: ProductColor[] }) {
  const [isPending, startTransition] = useTransition();
  const [colors, setColors] = useState<ProductColor[]>(productColors);
  const [newColor, setNewColor] = useState("");
  const [newHex, setNewHex] = useState(FALLBACK_COLOR_HEX);
  const hexSaveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [colorMsg, setColorMsg] = useState("");

  function save(updated: ProductColor[], msg: string) {
    startTransition(async () => {
      await saveProductColors(updated);
      setColorMsg(msg);
      setTimeout(() => setColorMsg(""), 3000);
    });
  }

  function handleAddColor() {
    const trimmed = newColor.trim();
    if (!trimmed) return;
    if (colors.some((c) => c.name.toLowerCase() === trimmed.toLowerCase())) {
      setColorMsg("Color already exists.");
      return;
    }
    const updated = [...colors, { name: trimmed, code: colorCode(trimmed), hex: newHex }];
    setColors(updated);
    setNewColor("");
    setNewHex(FALLBACK_COLOR_HEX);
    save(updated, "Colors updated successfully.");
  }

  // The picker fires on every drag step; save once it settles.
  function handleHexChange(name: string, hex: string) {
    const updated = colors.map((c) => (c.name === name ? { ...c, hex } : c));
    setColors(updated);
    clearTimeout(hexSaveTimer.current);
    hexSaveTimer.current = setTimeout(() => save(updated, "Color updated."), 400);
  }

  function handleRemoveColor(col: string) {
    const updated = colors.filter((c) => c.name !== col);
    setColors(updated);
    save(updated, "Color removed.");
  }

  return (
    <div className="card p-5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-base font-bold text-(--color-ink)">
            Standard Product Colors & Finishes
          </h3>
          <p className="text-xs text-(--color-muted) mt-0.5">
            Standard <strong>Walnut (WL)</strong>, <strong>Natural (NT)</strong> and <strong>Black (BL)</strong>, plus any finish you add below.
          </p>
        </div>
        <span className="rounded-full bg-emerald-50 px-2.5 py-1 text-xs font-bold text-emerald-800 border border-emerald-200">
          {colors.length} Finishes
        </span>
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        {colors.map((c) => (
          <div
            key={c.name}
            className="flex items-center gap-3 rounded-xl border border-slate-200 bg-white p-3 shadow-xs"
          >
            <label
              title={`Pick the ${c.name} swatch`}
              className="relative h-7 w-7 shrink-0 cursor-pointer rounded-full border border-black/20 shadow-xs"
              style={{ backgroundColor: c.hex }}
            >
              <input
                type="color"
                value={c.hex}
                onChange={(e) => handleHexChange(c.name, e.target.value)}
                aria-label={`${c.name} swatch color`}
                className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
              />
            </label>
            <div className="min-w-0 flex-1">
              <div className="flex items-center justify-between gap-2">
                <span className="text-sm font-bold text-(--color-ink)">{c.name}</span>
                <span className="flex items-center gap-1">
                  <span className="rounded font-mono font-black text-xs bg-slate-100 px-1.5 py-0.5 text-slate-800">
                    {c.code}
                  </span>
                  {!STANDARD_PRODUCT_COLORS.some((s) => s.name === c.name) && (
                    <button
                      type="button"
                      onClick={() => handleRemoveColor(c.name)}
                      title={`Remove ${c.name}`}
                      className="px-1 text-slate-400 hover:text-red-600 transition-colors"
                    >
                      &times;
                    </button>
                  )}
                </span>
              </div>
              <span className="text-[11px] text-(--color-muted)">Code suffix: /{c.code}</span>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-3 flex items-center gap-2 max-w-sm">
        <input
          type="color"
          value={newHex}
          onChange={(e) => setNewHex(e.target.value)}
          title="Swatch color for the new finish"
          aria-label="Swatch color for the new finish"
          className="h-8 w-10 shrink-0 cursor-pointer rounded border border-slate-300 bg-white p-0.5"
        />
        <input
          value={newColor}
          onChange={(e) => setNewColor(e.target.value)}
          placeholder="New color (e.g. Teak, Mahogany)"
          className="input text-xs py-1.5"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              handleAddColor();
            }
          }}
        />
        <button
          type="button"
          onClick={handleAddColor}
          disabled={isPending || !newColor.trim()}
          className="btn-primary text-xs py-1.5 whitespace-nowrap"
        >
          + Add Color
        </button>
      </div>
      {colorMsg && <p className="mt-2 text-xs font-medium text-emerald-700">{colorMsg}</p>}
    </div>
  );
}
