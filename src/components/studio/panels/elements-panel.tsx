"use client";

import { Slash, Spline } from "lucide-react";
import type { Editor } from "../editor";
import { SHAPES } from "../shapes";
import { cx, panelTitle } from "../studio-ui";

export function ElementsPanel({ ed }: { ed: Editor }) {
  return (
    <div className="space-y-4">
      <div>
        <div className={cx(panelTitle, "mb-2")}>Brand</div>
        <button
          type="button"
          title="Add the Lattice Lane logo"
          onClick={ed.addLogo}
          className="flex w-full items-center justify-center rounded-md border border-(--st-line) bg-white p-3 hover:border-(--st-accent)"
        >
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src="/brand/deck-logo.png" alt="Lattice Lane logo" className="h-16 w-auto" />
        </button>
      </div>
      <div>
        <div className={cx(panelTitle, "mb-2")}>Draw (MS Paint style)</div>
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            title="Draw straight line by dragging on canvas"
            onClick={() => ed.setActiveTool(ed.activeTool === "line" ? "select" : "line")}
            className={cx(
              "flex items-center gap-2 rounded-md border p-2.5 text-[12px] font-medium transition-colors",
              ed.activeTool === "line"
                ? "border-(--st-accent) bg-(--st-accent-soft) text-(--st-accent-strong)"
                : "border-(--st-line) bg-(--st-panel-2) hover:bg-(--st-hover)",
            )}
          >
            <Slash className="h-4 w-4 shrink-0 rotate-45" />
            <span>Draw Line</span>
          </button>
          <button
            type="button"
            title="Draw curved line by dragging on canvas"
            onClick={() => ed.setActiveTool(ed.activeTool === "curve" ? "select" : "curve")}
            className={cx(
              "flex items-center gap-2 rounded-md border p-2.5 text-[12px] font-medium transition-colors",
              ed.activeTool === "curve"
                ? "border-(--st-accent) bg-(--st-accent-soft) text-(--st-accent-strong)"
                : "border-(--st-line) bg-(--st-panel-2) hover:bg-(--st-hover)",
            )}
          >
            <Spline className="h-4 w-4 shrink-0" />
            <span>Draw Curve</span>
          </button>
        </div>
        <label className="mt-2.5 flex cursor-pointer items-center gap-2 px-1 text-[11.5px] text-(--st-muted) hover:text-(--st-text) select-none">
          <input
            type="checkbox"
            checked={ed.snapRightAngles}
            onChange={(e) => ed.setSnapRightAngles(e.target.checked)}
            className="h-3.5 w-3.5 rounded border-(--st-line) text-(--st-accent) focus:ring-0"
          />
          <span>Snap to right angles (0° / 90°)</span>
        </label>
      </div>
      <div>
        <div className={cx(panelTitle, "mb-2")}>Shapes</div>
        <div className="grid grid-cols-3 gap-2">
          {SHAPES.map((s) => (
            <button
              key={s.label}
              type="button"
              title={s.label}
              onClick={() => ed.add(s.make(ed.canvas))}
              className="flex aspect-square items-center justify-center rounded-md bg-(--st-panel-2) p-3 hover:bg-(--st-hover)"
            >
              <svg viewBox="0 0 48 48" className="h-full w-full fill-[#c8a97e] stroke-[#c8a97e]" dangerouslySetInnerHTML={{ __html: s.svg }} />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
