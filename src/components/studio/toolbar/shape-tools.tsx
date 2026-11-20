"use client";

import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { ColorButton, Divider, Popover, Slider } from "../studio-ui";

export function ShapeTools({ ed, l }: { ed: Editor; l: Extract<Layer, { kind: "rect" | "ellipse" | "polygon" | "star" }> }) {
  return (
    <>
      <ColorButton title="Fill colour" fill={l.fill} documentColors={ed.documentColors} onChange={(fill) => ed.tweak(l.id, { fill })} />
      <Popover
        title="Border"
        width={250}
        trigger={
          <>
            <span
              className="h-5 w-5 rounded border-[3px]"
              style={{ borderColor: l.strokeWidth > 0 ? l.stroke : "var(--st-muted)", borderStyle: l.strokeWidth > 0 ? "solid" : "dashed" }}
            />
            <span className="text-[12.5px]">Border</span>
          </>
        }
      >
        <div className="space-y-3">
          <Slider label="Border weight" value={l.strokeWidth} min={0} max={60} onChange={(strokeWidth) => ed.tweak(l.id, { strokeWidth })} />
          <label className="flex items-center justify-between text-[12px] text-(--st-muted)">
            Border colour
            <span className="relative h-7 w-10 cursor-pointer rounded-md border border-black/15" style={{ background: l.stroke }}>
              <input
                type="color"
                value={l.stroke}
                onChange={(e) => ed.tweak(l.id, { stroke: e.target.value, strokeWidth: l.strokeWidth || 4 })}
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </span>
          </label>
          {l.kind === "rect" && (
            <Slider
              label="Corner rounding"
              value={l.cornerRadius}
              min={0}
              max={Math.round(Math.min(l.width, l.height) / 2)}
              onChange={(cornerRadius) => ed.tweak(l.id, { cornerRadius })}
            />
          )}
          {l.kind === "polygon" && (
            <Slider label="Sides" value={l.sides} min={3} max={12} onChange={(sides) => ed.tweak(l.id, { sides })} />
          )}
          {l.kind === "star" && (
            <>
              <Slider label="Points" value={l.numPoints} min={3} max={20} onChange={(numPoints) => ed.tweak(l.id, { numPoints })} />
              <Slider
                label="Inner radius"
                value={Math.round((l.innerRadius / l.outerRadius) * 100)}
                min={10}
                max={95}
                suffix="%"
                onChange={(v) => ed.tweak(l.id, { innerRadius: (l.outerRadius * v) / 100 })}
              />
            </>
          )}
        </div>
      </Popover>
      <Divider />
    </>
  );
}
