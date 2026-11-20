"use client";

import { fillCss, type Layer } from "@/lib/hamper-canvas";
import { isLine, isShape, type MenuItem } from "./menu-items";

const BRAND_COLORS = ["#2c332f", "#54655b", "#ddcf8b", "#b08d57", "#c0392b", "#ffffff"];

export function ContextMenu({
  x,
  y,
  items,
  selection,
  documentColors,
  onApplyColor,
  onClose,
}: {
  x: number;
  y: number;
  items: MenuItem[];
  selection?: Layer[];
  documentColors?: string[];
  onApplyColor?: (color: string) => void;
  onClose: () => void;
}) {
  const hasShape = selection?.some(isShape);
  const isOnlyLine = selection && selection.length > 0 && selection.every(isLine);
  const firstShape = selection?.find((l) => "fill" in l || "stroke" in l);
  const currentPreview = firstShape
    ? "fill" in firstShape && !isOnlyLine
      ? fillCss(firstShape.fill)
      : "stroke" in firstShape
        ? firstShape.stroke
        : "#54655b"
    : "#54655b";
  const currentHex = firstShape
    ? "stroke" in firstShape && isOnlyLine
      ? firstShape.stroke
      : "fill" in firstShape && firstShape.fill.type === "solid"
        ? firstShape.fill.color
        : "#54655b"
    : "#54655b";

  const quickColors = [...new Set([...BRAND_COLORS, ...(documentColors ?? []).slice(0, 3)])].slice(0, 7);
  const apply = (c: string) => {
    onApplyColor?.(c);
    onClose();
  };

  // Keep the menu on screen near the right and bottom edges.
  const left = Math.min(x, window.innerWidth - 240);
  const top = Math.min(y, Math.max(10, window.innerHeight - items.length * 30 - (hasShape ? 70 : 0) - 16));
  return (
    <div
      role="menu"
      className="studio fixed z-70 w-56 rounded-lg border border-(--st-line) bg-(--st-panel) p-1 shadow-xl shadow-[#2c332f]/15"
      style={{ left, top }}
      onPointerDown={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {hasShape && onApplyColor && (
        <div className="border-b border-(--st-line) px-2.5 py-2">
          <div className="mb-1.5 flex items-center justify-between text-[10.5px] font-semibold uppercase tracking-wider text-(--st-muted)">
            <span>{isOnlyLine ? "Line colour" : "Shape colour"}</span>
            <span className="h-3.5 w-3.5 rounded-full border border-black/20" style={{ background: currentPreview }} />
          </div>
          <div className="flex items-center gap-1.5">
            {quickColors.map((c) => (
              <button
                key={c}
                type="button"
                title={c}
                onClick={() => apply(c)}
                className="h-5 w-5 rounded-full border border-black/15 transition-transform hover:scale-110"
                style={{ background: c }}
              />
            ))}
            <label
              title="Pick custom colour"
              className="relative flex h-5 w-5 cursor-pointer items-center justify-center rounded-full border border-black/15 transition-transform hover:scale-110"
              style={{ background: "conic-gradient(red, yellow, lime, cyan, blue, magenta, red)" }}
            >
              <input
                type="color"
                value={currentHex}
                onChange={(e) => apply(e.target.value)}
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </label>
          </div>
        </div>
      )}
      {items.map((item, i) =>
        item === null ? (
          <div key={i} className="my-1 h-px bg-(--st-line)" />
        ) : (
          <button
            key={item[0]}
            type="button"
            role="menuitem"
            disabled={!item[2]}
            onClick={() => {
              item[2]?.();
              onClose();
            }}
            className="flex w-full items-center justify-between rounded px-2.5 py-1.5 text-left text-[13px] hover:bg-(--st-hover) disabled:opacity-35 disabled:hover:bg-transparent"
          >
            <span className="flex items-center gap-2">
              {item[0].startsWith("Edit fill") && (
                <span className="h-3 w-3 rounded-full border border-black/15" style={{ background: currentPreview }} />
              )}
              {item[0].startsWith("Edit border") && (
                <span className="h-3 w-3 rounded border border-black/30" style={{ borderColor: currentHex }} />
              )}
              {item[0].startsWith("Edit line") && (
                <span className="h-0.5 w-3.5 rounded" style={{ background: currentHex }} />
              )}
              {item[0]}
            </span>
            <span className="text-[11px] text-(--st-muted)">{item[1]}</span>
          </button>
        ),
      )}
    </div>
  );
}
