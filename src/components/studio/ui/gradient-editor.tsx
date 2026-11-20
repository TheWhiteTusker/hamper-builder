"use client";

import { fillCss, type Fill } from "@/lib/hamper-canvas";
import { Slider, cx, toolBtn, toolBtnActive } from "./base";

/** Gradient preview, start/end colours and angle. `first` seeds a gradient from a solid fill. */
export function GradientEditor({ fill, first, onChange }: { fill: Fill; first: string; onChange: (fill: Fill) => void }) {
  const g: Extract<Fill, { type: "linear" }> =
    fill.type === "linear" ? fill : { type: "linear", from: first, to: "#ffffff", angle: 90 };
  return (
    <>
      <div className="h-8 rounded-md border border-black/10" style={{ background: fillCss(g) }} />
      <div className="flex items-center gap-2">
        {(["from", "to"] as const).map((k) => (
          <label key={k} className="flex flex-1 items-center gap-2 text-[12px] text-(--st-muted)">
            <span
              className="relative h-7 w-7 shrink-0 cursor-pointer rounded-md border border-black/10"
              style={{ background: g[k] }}
            >
              <input
                type="color"
                value={g[k]}
                onChange={(e) => onChange({ ...g, [k]: e.target.value })}
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </span>
            {k === "from" ? "Start" : "End"}
          </label>
        ))}
      </div>
      <Slider label="Angle" value={g.angle} min={0} max={359} suffix="°" onChange={(angle) => onChange({ ...g, angle })} />
      <div className="flex gap-1">
        {[0, 45, 90, 135, 180, 270].map((a) => (
          <button
            key={a}
            type="button"
            onClick={() => onChange({ ...g, angle: a })}
            className={cx(toolBtn, "h-7 flex-1 px-0 text-[11px]", g.angle === a && toolBtnActive)}
          >
            {a}°
          </button>
        ))}
      </div>
    </>
  );
}
