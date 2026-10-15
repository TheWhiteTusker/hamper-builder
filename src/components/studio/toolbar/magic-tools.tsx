"use client";

import { Layers } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { looksWeak } from "../stage/use-magic-layers";
import { Popover, accentBtn, cx } from "../studio-ui";

const icon = "h-4 w-4";
const note = "text-[11.5px] leading-snug text-[var(--st-muted)]";

/** Magic layers: the on/off switch for this computer, and the button that splits the image. */
export function MagicTools({ ed, l }: { ed: Editor; l: Extract<Layer, { kind: "image" }> }) {
  const step = ed.magicStep;
  return (
    <Popover
      title="Magic layers"
      width={280}
      trigger={<><Layers className={icon} /> {step ? "Splitting…" : "Magic layers"}</>}
    >
      {() => (
        <div className="space-y-3">
          <label className="flex cursor-pointer select-none items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              role="switch"
              checked={ed.magicEnabled}
              disabled={!!step}
              onChange={(e) => ed.setMagicEnabled(e.target.checked)}
              className="h-3.5 w-3.5 rounded border-[var(--st-line)] text-[var(--st-accent)] focus:ring-0"
            />
            <span>Enable Magic layers on this computer</span>
          </label>
          {ed.magicEnabled ? (
            <p className={note}>
              Splits this image into a background, a cut-out for each object and editable text. Runs on this computer;
              the first run downloads the AI models (tens of MB, then cached). Undo brings the original back.
            </p>
          ) : (
            <p className={note}>
              Off on this computer, so nothing is downloaded.
              {looksWeak() && " It started off because this computer looks slow for it; you can still turn it on."}
            </p>
          )}
          {step && (
            <div className="space-y-1">
              <p className="text-[12px]">{step.label}…</p>
              {step.progress !== undefined && (
                <div className="h-1.5 overflow-hidden rounded bg-[var(--st-line)]">
                  <div className="h-full bg-[var(--st-accent)]" style={{ width: `${Math.round(step.progress * 100)}%` }} />
                </div>
              )}
            </div>
          )}
          <button
            type="button"
            className={cx(accentBtn, "w-full")}
            disabled={!ed.magicEnabled || !!step}
            onClick={() => ed.magicSplit(l.id)}
          >
            <Layers className={icon} />
            {step ? "Splitting…" : "Split into layers"}
          </button>
        </div>
      )}
    </Popover>
  );
}
