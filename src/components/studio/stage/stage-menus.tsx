"use client";

import { useState } from "react";
import { X } from "lucide-react";
import type { Fill } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { ColorPanel } from "../studio-ui";
import { ContextMenu } from "./context-menu";
import { hasFill, isLine, menuItems } from "./menu-items";
import type { CanvasDoc } from "./use-canvas-doc";

type ColorEdit = { mode: "fill" | "stroke"; ids: string[]; x: number; y: number };

/** The right-click menu and the fill / border colour dialog it opens. */
export function StageMenus({
  doc,
  ed,
  zoomFit,
  recenter,
}: {
  doc: CanvasDoc;
  ed: Editor;
  zoomFit: () => void;
  recenter: () => void;
}) {
  const [colorEdit, setColorEdit] = useState<ColorEdit | null>(null);
  const { menu, selection, selectedIds, selected, patchEach, canvas } = doc;
  const strokeWidth = Math.max(1, selected && "strokeWidth" in selected ? selected.strokeWidth : 4);

  function applyColor(c: string) {
    if (selection.length > 0 && selection.every(isLine)) patchEach(selectedIds, { stroke: c, strokeWidth });
    else if (selection.some(hasFill)) patchEach(selectedIds, { fill: { type: "solid", color: c } });
    else if (selection.some((l) => "stroke" in l)) patchEach(selectedIds, { stroke: c, strokeWidth });
  }

  const target = colorEdit && canvas.layers.find((l) => colorEdit.ids.includes(l.id));
  const current: Fill =
    colorEdit?.mode === "fill" && target && "fill" in target
      ? target.fill
      : target && "stroke" in target
        ? { type: "solid", color: target.stroke }
        : { type: "solid", color: "#54655b" };

  return (
    <>
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          selection={selection}
          documentColors={ed.documentColors}
          onApplyColor={applyColor}
          items={menuItems({
            doc,
            ed,
            snapRightAngles: ed.snapRightAngles,
            toggleSnap: () => ed.setSnapRightAngles((s) => !s),
            zoomFit,
            recenter,
            editColor: (mode) => setColorEdit({ mode, ids: selectedIds, x: menu.x, y: menu.y }),
          })}
          onClose={() => doc.setMenu(null)}
        />
      )}

      {colorEdit && (
        <div
          role="dialog"
          aria-label="Edit colours"
          className="studio fixed z-75 w-72 rounded-lg border border-(--st-line) bg-(--st-panel) p-3 shadow-2xl shadow-[#2c332f]/25"
          style={{
            left: Math.min(colorEdit.x, window.innerWidth - 300),
            top: Math.min(colorEdit.y, Math.max(10, window.innerHeight - 440)),
          }}
          onPointerDown={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <div className="mb-3 flex items-center justify-between border-b border-(--st-line) pb-2">
            <span className="text-[12px] font-semibold">
              {colorEdit.mode === "fill" ? "Fill colour" : "Line / Border colour"}
            </span>
            <button
              type="button"
              onClick={() => setColorEdit(null)}
              className="rounded p-1 text-(--st-muted) hover:bg-(--st-hover) hover:text-(--st-text)"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
          <ColorPanel
            fill={current}
            documentColors={ed.documentColors}
            allowGradient={colorEdit.mode === "fill"}
            onChange={(next) => {
              if (colorEdit.mode === "fill") return patchEach(colorEdit.ids, { fill: next }, "shape-color");
              const sw = target && "strokeWidth" in target ? target.strokeWidth : 4;
              const color = next.type === "solid" ? next.color : next.from;
              patchEach(colorEdit.ids, { stroke: color, strokeWidth: Math.max(1, sw) }, "shape-color");
            }}
          />
        </div>
      )}
    </>
  );
}
