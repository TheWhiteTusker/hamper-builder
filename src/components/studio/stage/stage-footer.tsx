"use client";

import { Keyboard, Minus, Plus } from "lucide-react";
import { PageSizeInputs, Popover, ToolButton, cx, panelTitle } from "../studio-ui";
import { ZOOM_MAX, ZOOM_MIN } from "./konva-shapes";
import type { StageView } from "./use-stage-view";
import { STUDIO_SHORTCUTS } from "../shortcut-list";

/** Shortcuts, page size, layer count and zoom controls. */
export function StageFooter({
  view,
  width,
  height,
  layerCount,
  onResize,
}: {
  view: StageView;
  width: number;
  height: number;
  layerCount: number;
  onResize: (w: number, h: number) => void;
}) {
  const { zoom, zoomTo } = view;
  return (
    <footer className="flex h-10 shrink-0 items-center gap-2 border-t border-(--st-line) bg-(--st-panel) px-3">
      <Popover title="Keyboard shortcuts" width={330} trigger={<Keyboard className="h-4 w-4" />}>
        <div className={cx(panelTitle, "mb-2")}>Keyboard shortcuts</div>
        <dl className="space-y-1.5">
          {STUDIO_SHORTCUTS.map(([what, keys]) => (
            <div key={what} className="flex justify-between gap-3 text-[12px]">
              <dt className="text-(--st-muted)">{what}</dt>
              <dd className="text-right font-mono text-[11px]">{keys}</dd>
            </div>
          ))}
        </dl>
      </Popover>
      <PageSizeInputs width={width} height={height} onChange={onResize} />
      <span className="text-[12px] text-(--st-muted)">
        · {layerCount} layer{layerCount === 1 ? "" : "s"}
      </span>
      <div className="ml-auto flex items-center gap-1">
        <ToolButton title="Zoom out (Ctrl+-)" className="h-7 w-7 px-0" onClick={() => zoomTo((z) => z / 1.25)}>
          <Minus className="h-3.5 w-3.5" />
        </ToolButton>
        <input
          type="range"
          aria-label="Zoom"
          min={Math.log(ZOOM_MIN)}
          max={Math.log(ZOOM_MAX)}
          step={0.01}
          value={Math.log(zoom)}
          onChange={(e) => zoomTo(() => Math.exp(Number(e.target.value)))}
          className="w-32"
        />
        <ToolButton title="Zoom in (Ctrl+=)" className="h-7 w-7 px-0" onClick={() => zoomTo((z) => z * 1.25)}>
          <Plus className="h-3.5 w-3.5" />
        </ToolButton>
        <span className="w-12 text-right text-[12px] tabular-nums">{Math.round(zoom * 100)}%</span>
        <ToolButton title="Fit page to screen (Ctrl+0)" active={view.view.zoom === null} onClick={view.zoomFit}>
          Fit
        </ToolButton>
        <ToolButton
          title="Re-centre page at current size"
          active={view.view.zoom !== null && view.view.panX === 0 && view.view.panY === 0}
          onClick={view.recenter}
        >
          Re-centre
        </ToolButton>
      </div>
    </footer>
  );
}
