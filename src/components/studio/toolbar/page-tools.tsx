"use client";

import { ImagePlus, Magnet, Slash, Spline, Trash2 } from "lucide-react";
import type { Editor } from "../editor";
import { ColorButton, Divider, PageSizeInputs, ToolButton, cx } from "../studio-ui";

const icon = "h-4 w-4";

/** Turns right-angle snapping for lines on and off. */
export function SnapToggle({ ed }: { ed: Editor }) {
  return (
    <ToolButton
      title={ed.snapRightAngles ? "Snap right angles: ON (click to disable)" : "Snap right angles: OFF (click to enable)"}
      active={ed.snapRightAngles}
      onClick={() => ed.setSnapRightAngles((s) => !s)}
    >
      <Magnet className="h-4 w-4" />
      <span>Snap 90°</span>
    </ToolButton>
  );
}

export function PageTools({ ed }: { ed: Editor }) {
  const bg = ed.canvas.background;
  return (
    <>
      <span className="mr-1 text-[12px] text-(--st-muted)">Page size</span>
      <PageSizeInputs
        width={ed.canvas.width}
        height={ed.canvas.height}
        onChange={(width, height) => ed.change((c) => ({ ...c, width, height }))}
      />
      <Divider />
      <ColorButton
        title="Background colour"
        fill={bg.fill}
        documentColors={ed.documentColors}
        onChange={(fill) => ed.change((c) => ({ ...c, background: { ...c.background, fill } }), "bg-fill")}
      />
      <label className={cx("inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md px-2 hover:bg-(--st-hover)")}>
        <ImagePlus className={icon} />
        {bg.image_url ? "Replace background image" : "Background image"}
        <input
          type="file"
          accept="image/*"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) ed.uploadBackground(f);
            e.target.value = "";
          }}
        />
      </label>
      {bg.image_url && (
        <ToolButton
          title="Remove background image"
          onClick={() => ed.change((c) => ({ ...c, background: { ...c.background, image_url: null } }))}
        >
          <Trash2 className={icon} />
        </ToolButton>
      )}
      <Divider />
      <ToolButton
        title="Draw straight line (MS Paint style)"
        active={ed.activeTool === "line"}
        onClick={() => ed.setActiveTool(ed.activeTool === "line" ? "select" : "line")}
      >
        <Slash className="h-4 w-4 rotate-45" />
        <span>Line</span>
      </ToolButton>
      <ToolButton
        title="Draw curved line (MS Paint style)"
        active={ed.activeTool === "curve"}
        onClick={() => ed.setActiveTool(ed.activeTool === "curve" ? "select" : "curve")}
      >
        <Spline className="h-4 w-4" />
        <span>Curve</span>
      </ToolButton>
      <SnapToggle ed={ed} />
      <span className="ml-auto text-[12px] text-(--st-muted)">
        Click to select · Shift+click or drag a box to select several · right-click for more
      </span>
    </>
  );
}
