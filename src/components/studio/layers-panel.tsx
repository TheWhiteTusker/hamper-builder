"use client";

import { useState } from "react";
import { Eye, EyeOff, GripVertical, Layers, Lock, LockOpen } from "lucide-react";
import { layerLabel } from "@/lib/hamper-canvas";
import type { Editor } from "./editor";
import { cx, ToolButton } from "./studio-ui";
import { Thumb } from "./panels/layer-thumb";

export { TransformPanel } from "./panels/transform-panel";

export function LayersPanel({ ed }: { ed: Editor }) {
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const layers = ed.canvas.layers;
  // Top of the list = top of the stack, as in After Effects and Canva.
  const shown = [...layers].reverse();

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <header className="flex h-10 shrink-0 items-center gap-2 border-b border-(--st-line) px-3">
        <Layers className="h-4 w-4 text-(--st-muted)" />
        <span className="font-semibold">Layers</span>
        <span className="ml-auto text-[11px] text-(--st-muted)">{layers.length}</span>
      </header>

      <ul className="min-h-0 flex-1 overflow-y-auto py-1" onDragLeave={() => setOverId(null)}>
        {shown.length === 0 && (
          <li className="px-3 py-6 text-center text-[12px] text-(--st-muted)">
            Nothing here yet. Add products, text or shapes from the left.
          </li>
        )}
        {shown.map((l) => {
          const selected = ed.selectedIds.includes(l.id);
          return (
            <li
              key={l.id}
              draggable={renaming !== l.id}
              onDragStart={(e) => {
                setDragId(l.id);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragEnd={() => {
                setDragId(null);
                setOverId(null);
              }}
              onDragOver={(e) => {
                if (!dragId) return;
                e.preventDefault();
                setOverId(l.id);
              }}
              onDrop={(e) => {
                e.preventDefault();
                if (dragId) ed.reorder(dragId, layers.findIndex((x) => x.id === l.id));
                setDragId(null);
                setOverId(null);
              }}
              onClick={(e) => {
                if (e.shiftKey && ed.selectedIds.length) {
                  // Range from the last-picked layer to this one, in stacking order.
                  const anchor = layers.findIndex((x) => x.id === ed.selectedIds[ed.selectedIds.length - 1]);
                  const here = layers.findIndex((x) => x.id === l.id);
                  const [a, b] = anchor < here ? [anchor, here] : [here, anchor];
                  ed.selectMany(layers.slice(a, b + 1).map((x) => x.id));
                } else if (e.ctrlKey || e.metaKey) {
                  ed.toggleSelect(l.id);
                } else {
                  ed.select(l.id);
                }
              }}
              className={cx(
                "group mx-1 flex cursor-default items-center gap-2 rounded-md border-t-2 border-transparent px-1.5 py-1",
                selected ? "bg-(--st-accent-soft)" : "hover:bg-(--st-hover)",
                overId === l.id && dragId !== l.id && "border-t-(--st-accent)",
                dragId === l.id && "opacity-40",
                !l.visible && "opacity-50",
              )}
            >
              <GripVertical className="h-4 w-4 shrink-0 cursor-grab text-(--st-muted) opacity-0 group-hover:opacity-100" />
              <Thumb l={l} />
              {renaming === l.id ? (
                <input
                  autoFocus
                  defaultValue={layerLabel(l)}
                  className="h-6 min-w-0 flex-1 rounded border border-(--st-accent) bg-(--st-panel-2) px-1 text-[12px] outline-none"
                  onFocus={(e) => e.target.select()}
                  onBlur={(e) => {
                    ed.patch(l.id, { name: e.target.value.trim() || undefined });
                    setRenaming(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                    if (e.key === "Escape") setRenaming(null);
                  }}
                />
              ) : (
                <span
                  className="min-w-0 flex-1 truncate text-[12.5px]"
                  title="Double-click to rename"
                  onDoubleClick={() => setRenaming(l.id)}
                >
                  {layerLabel(l)}
                </span>
              )}
              <ToolButton
                title={l.locked ? "Unlock" : "Lock"}
                className={cx("h-6 w-6 px-0", !l.locked && "opacity-0 group-hover:opacity-100")}
                onClick={(e) => {
                  e.stopPropagation();
                  ed.patch(l.id, { locked: !l.locked });
                }}
              >
                {l.locked ? <Lock className="h-3.5 w-3.5" /> : <LockOpen className="h-3.5 w-3.5" />}
              </ToolButton>
              <ToolButton
                title={l.visible ? "Hide" : "Show"}
                className={cx("h-6 w-6 px-0", l.visible && "opacity-0 group-hover:opacity-100")}
                onClick={(e) => {
                  e.stopPropagation();
                  ed.patch(l.id, { visible: !l.visible });
                }}
              >
                {l.visible ? <Eye className="h-3.5 w-3.5" /> : <EyeOff className="h-3.5 w-3.5" />}
              </ToolButton>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
