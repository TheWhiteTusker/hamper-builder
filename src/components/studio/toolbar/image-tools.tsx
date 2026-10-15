"use client";

import { useState } from "react";
import { FlipHorizontal2, FlipVertical2, Maximize2, Replace, Scissors } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { Divider, Popover, Slider, ToolButton, accentBtn, cx } from "../studio-ui";
import { MagicTools } from "./magic-tools";

const icon = "h-4 w-4";

export function ImageTools({ ed, l }: { ed: Editor; l: Extract<Layer, { kind: "image" }> }) {
  const [tolerance, setTolerance] = useState(40);
  return (
    <>
      <ToolButton title="Replace with another image" active={ed.swapId === l.id} onClick={() => ed.startSwap(ed.swapId === l.id ? null : l.id)}>
        <Replace className={icon} /> Swap
      </ToolButton>
      {l.product_id && (
        <Popover title="Remove background" width={260} trigger={<><Scissors className={icon} /> BG Remover</>}>
          {(close) => (
            <div className="space-y-3">
              <Slider label="Tolerance" value={tolerance} min={5} max={120} onChange={setTolerance} />
              <p className="text-[11.5px] leading-snug text-[var(--st-muted)]">
                Works on plain backdrops. Raise it if some backdrop is left; lower it if the product gets cut. Undo brings the original back.
              </p>
              <button
                type="button"
                className={cx(accentBtn, "w-full")}
                disabled={ed.removing}
                onClick={() => {
                  ed.cutOut(l.id, tolerance);
                  close();
                }}
              >
                <Scissors className={icon} />
                {ed.removing ? "Removing…" : "Remove background"}
              </button>
            </div>
          )}
        </Popover>
      )}
      <MagicTools ed={ed} l={l} />
      <Popover title="Flip" width={170} trigger={<><FlipHorizontal2 className={icon} /> Flip</>}>
        {(close) => (
          <div className="space-y-1">
            {(
              [
                ["x", "Flip horizontal", FlipHorizontal2],
                ["y", "Flip vertical", FlipVertical2],
              ] as const
            ).map(([axis, label, Icon]) => (
              <button
                key={axis}
                type="button"
                className="flex w-full items-center gap-2 rounded px-2 py-1.5 hover:bg-[var(--st-hover)]"
                onClick={() => {
                  ed.flip(l.id, axis);
                  close();
                }}
              >
                <Icon className={icon} /> {label}
              </button>
            ))}
          </div>
        )}
      </Popover>
      <ToolButton title="Shrink to fit inside the page and centre it" onClick={() => ed.fitToPage(l.id)}>
        <Maximize2 className={icon} /> Fit to page
      </ToolButton>
      <Divider />
    </>
  );
}
