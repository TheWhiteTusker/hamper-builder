"use client";

import { useState } from "react";
import { Link2, Unlink2 } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { cx, panelTitle, ScrubNumber, ToolButton } from "../studio-ui";

/** The unscaled box size of a layer, which scaleX/scaleY multiply. */
function baseSize(l: Layer) {
  switch (l.kind) {
    case "image":
    case "rect":
      return { w: l.width, h: l.height };
    case "text":
      return { w: l.width, h: l.fontSize * l.lineHeight * Math.max(1, l.text.split("\n").length) };
    case "ellipse":
      return { w: l.radiusX * 2, h: l.radiusY * 2 };
    case "polygon":
      return { w: l.radius * 2, h: l.radius * 2 };
    case "star":
      return { w: l.outerRadius * 2, h: l.outerRadius * 2 };
    case "line":
    case "curve": {
      const xs = l.points.filter((_, i) => i % 2 === 0);
      const ys = l.points.filter((_, i) => i % 2 === 1);
      return {
        w: Math.max(20, Math.max(...xs) - Math.min(...xs)),
        h: Math.max(20, Math.max(...ys) - Math.min(...ys)),
      };
    }
  }
}

export function TransformPanel({ ed }: { ed: Editor }) {
  const [linked, setLinked] = useState(true);
  const l = ed.selected;

  return (
    <section className="shrink-0 border-t border-(--st-line) p-3">
      <div className={cx(panelTitle, "mb-2")}>Transform</div>
      {ed.selection.length > 1 ? (
        <p className="text-[12px] text-(--st-muted)">
          {ed.selection.length} layers selected. Drag any of them to move them together, or use Align in the toolbar.
        </p>
      ) : !l ? (
        <p className="text-[12px] text-(--st-muted)">
          Select a layer to see its position and size. Shift-click for a range, Ctrl-click to add one.
        </p>
      ) : (
        (() => {
          const { w, h } = baseSize(l);
          const W = w * Math.abs(l.scaleX);
          const H = h * Math.abs(l.scaleY);
          const off = l.locked;
          const setW = (v: number) => {
            const sx = Math.max(1, v) / w;
            const ratio = sx / Math.abs(l.scaleX);
            ed.tweak(l.id, {
              scaleX: Math.sign(l.scaleX) * sx,
              ...(linked ? { scaleY: l.scaleY * ratio } : {}),
            });
          };
          const setH = (v: number) => {
            const sy = Math.max(1, v) / h;
            const ratio = sy / Math.abs(l.scaleY);
            ed.tweak(l.id, {
              scaleY: Math.sign(l.scaleY) * sy,
              ...(linked ? { scaleX: l.scaleX * ratio } : {}),
            });
          };
          return (
            <div className="grid grid-cols-[1fr_1fr_auto] items-center gap-x-2 gap-y-1">
              <ScrubNumber label="X" value={l.x} disabled={off} onChange={(x) => ed.tweak(l.id, { x })} />
              <ScrubNumber label="Y" value={l.y} disabled={off} onChange={(y) => ed.tweak(l.id, { y })} />
              <span />
              <ScrubNumber label="W" value={W} min={1} disabled={off} onChange={setW} />
              <ScrubNumber label="H" value={H} min={1} disabled={off} onChange={setH} />
              <ToolButton
                title={linked ? "Unlink width and height" : "Link width and height"}
                active={linked}
                className="h-6 w-6 px-0"
                onClick={() => setLinked((v) => !v)}
              >
                {linked ? <Link2 className="h-3.5 w-3.5" /> : <Unlink2 className="h-3.5 w-3.5" />}
              </ToolButton>
              <ScrubNumber
                label="↻"
                value={l.rotation}
                suffix="°"
                disabled={off}
                onChange={(rotation) => ed.tweak(l.id, { rotation: ((rotation % 360) + 360) % 360 })}
              />
              <ScrubNumber
                label="◐"
                value={l.opacity * 100}
                min={0}
                max={100}
                suffix="%"
                onChange={(v) => ed.tweak(l.id, { opacity: v / 100 })}
              />
            </div>
          );
        })()
      )}
    </section>
  );
}
