"use client";

import { Spline } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { SnapToggle } from "./page-tools";
import { ColorButton, Divider, Popover, Slider, cx, panelTitle } from "../studio-ui";

const icon = "h-4 w-4";

export function LineTools({ ed, l }: { ed: Editor; l: Extract<Layer, { kind: "line" | "curve" }> }) {
  const isCurve = l.kind === "curve";

  const handleCurvatureChange = (newCurvature: number) => {
    if (l.kind !== "curve") return;
    const endX = l.points[4] ?? 300;
    const endY = l.points[5] ?? 0;
    const midX = endX / 2;
    const midY = endY / 2;
    const dx = endX;
    const dy = endY;
    const len = Math.hypot(dx, dy) || 1;
    const nx = -dy / len;
    const ny = dx / len;
    const offset = (len * newCurvature) / 2;
    const newCx = midX + nx * offset;
    const newCy = midY + ny * offset;

    ed.tweak(l.id, {
      curvature: newCurvature,
      points: [0, 0, Math.round(newCx), Math.round(newCy), endX, endY],
    });
  };

  const flipCurve = () => {
    if (l.kind !== "curve") return;
    handleCurvatureChange(-l.curvature);
  };

  const straighten = () => {
    if (l.kind !== "curve") return;
    const endX = l.points[4] ?? 300;
    const endY = l.points[5] ?? 0;
    ed.tweak(l.id, {
      curvature: 0,
      points: [0, 0, Math.round(endX / 2), Math.round(endY / 2), endX, endY],
    });
  };

  return (
    <>
      <ColorButton
        title="Line colour"
        fill={{ type: "solid", color: l.stroke }}
        documentColors={ed.documentColors}
        onChange={(fill) => ed.tweak(l.id, { stroke: fill.type === "solid" ? fill.color : fill.from })}
      />

      <Popover
        title="Line style & weight"
        width={250}
        trigger={
          <>
            <span
              className="h-1.5 w-6 rounded"
              style={{
                background: l.stroke,
                borderTop: l.dash && l.dash.length ? "2px dashed" : "none",
              }}
            />
            <span className="text-[12.5px]">{l.strokeWidth}px</span>
          </>
        }
      >
        <div className="space-y-3">
          <Slider
            label="Line thickness"
            value={l.strokeWidth}
            min={1}
            max={60}
            suffix="px"
            onChange={(strokeWidth) => ed.tweak(l.id, { strokeWidth })}
          />

          <div>
            <div className={cx(panelTitle, "mb-1.5")}>Line style</div>
            <div className="grid grid-cols-3 gap-1">
              {[
                { label: "Solid", dash: undefined },
                { label: "Dashed", dash: [14, 10] },
                { label: "Dotted", dash: [4, 6] },
              ].map((s) => {
                const active = !s.dash ? !l.dash || l.dash.length === 0 : Boolean(l.dash && l.dash.length > 0 && l.dash[0] === s.dash[0]);
                return (
                  <button
                    key={s.label}
                    type="button"
                    onClick={() => ed.tweak(l.id, { dash: s.dash })}
                    className={cx(
                      "h-7 rounded border text-[12px] transition-colors",
                      active
                        ? "border-(--st-accent) bg-(--st-accent-soft) text-(--st-accent-strong) font-medium"
                        : "border-(--st-line) bg-(--st-panel-2) hover:bg-(--st-hover) text-(--st-text)",
                    )}
                  >
                    {s.label}
                  </button>
                );
              })}
            </div>
          </div>

          <div>
            <div className={cx(panelTitle, "mb-1.5")}>Line ends</div>
            <div className="grid grid-cols-3 gap-1">
              {(["round", "butt", "square"] as const).map((cap) => (
                <button
                  key={cap}
                  type="button"
                  onClick={() => ed.tweak(l.id, { lineCap: cap })}
                  className={cx(
                    "h-7 rounded border text-[12px] capitalize transition-colors",
                    l.lineCap === cap
                      ? "border-(--st-accent) bg-(--st-accent-soft) text-(--st-accent-strong) font-medium"
                      : "border-(--st-line) bg-(--st-panel-2) hover:bg-(--st-hover) text-(--st-text)",
                  )}
                >
                  {cap === "butt" ? "flat" : cap}
                </button>
              ))}
            </div>
          </div>
        </div>
      </Popover>

      {isCurve && (
        <Popover
          title="Curvature"
          width={240}
          trigger={
            <>
              <Spline className={icon} />
              <span className="text-[12.5px]">Curvature</span>
            </>
          }
        >
          <div className="space-y-3">
            <Slider
              label="Bend amount"
              value={Math.round(l.curvature * 100)}
              min={-200}
              max={200}
              suffix="%"
              onChange={(v) => handleCurvatureChange(v / 100)}
            />
            <div className="flex gap-2">
              <button
                type="button"
                onClick={flipCurve}
                className="flex-1 rounded border border-(--st-line) bg-(--st-panel-2) py-1 text-[12px] hover:bg-(--st-hover)"
              >
                Flip curve
              </button>
              <button
                type="button"
                onClick={straighten}
                className="flex-1 rounded border border-(--st-line) bg-(--st-panel-2) py-1 text-[12px] hover:bg-(--st-hover)"
              >
                Straighten
              </button>
            </div>
          </div>
        </Popover>
      )}

      <SnapToggle ed={ed} />

      <Divider />
    </>
  );
}
