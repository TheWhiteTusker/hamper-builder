"use client";

import { ImagePlus, Loader2, Trash2 } from "lucide-react";
import { CANVAS_PRESETS } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { ColorPanel, cx, panelTitle, toolBtn } from "../studio-ui";

export function BackgroundPanel({ ed }: { ed: Editor }) {
  const bg = ed.canvas.background;
  return (
    <div className="space-y-5">
      <ColorPanel
        fill={bg.fill}
        documentColors={ed.documentColors}
        onChange={(fill) => ed.change((c) => ({ ...c, background: { ...c.background, fill } }), "bg-fill")}
      />

      <div>
        <div className={cx(panelTitle, "mb-2")}>Background image</div>
        {bg.image_url && (
          <div className="checkerboard mb-2 aspect-video overflow-hidden rounded-md border border-(--st-line)">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={bg.image_url} alt="" className="h-full w-full object-cover" />
          </div>
        )}
        <div className="flex gap-2">
          <label
            className={cx(
              toolBtn,
              "flex-1 cursor-pointer border border-(--st-line)",
              ed.uploading && "pointer-events-none opacity-60",
            )}
          >
            {ed.uploading ? <Loader2 className="h-4 w-4 animate-spin" /> : <ImagePlus className="h-4 w-4" />}
            {ed.uploading ? "Uploading…" : bg.image_url ? "Replace" : "Upload image"}
            <input
              type="file"
              accept="image/*"
              disabled={ed.uploading}
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) ed.uploadBackground(f);
                e.target.value = "";
              }}
            />
          </label>
          {bg.image_url && (
            <button
              type="button"
              title="Remove background image"
              disabled={ed.uploading}
              className={cx(toolBtn, "border border-(--st-line)", ed.uploading && "opacity-50")}
              onClick={() => ed.change((c) => ({ ...c, background: { ...c.background, image_url: null } }))}
            >
              <Trash2 className="h-4 w-4" />
            </button>
          )}
        </div>
        <p className="mt-1.5 text-[11px] text-(--st-muted)">Fills the page, e.g. an empty basket or box.</p>
      </div>

      <div>
        <div className={cx(panelTitle, "mb-2")}>Page size</div>
        <div className="space-y-1">
          {CANVAS_PRESETS.map((p) => {
            const on = p.width === ed.canvas.width && p.height === ed.canvas.height;
            return (
              <button
                key={p.label}
                type="button"
                onClick={() => ed.change((c) => ({ ...c, width: p.width, height: p.height }))}
                className={cx(
                  "flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left hover:bg-(--st-hover)",
                  on && "bg-(--st-accent-soft)",
                )}
              >
                <span className="flex h-7 w-7 items-center justify-center">
                  <span
                    className="border border-(--st-muted)"
                    style={{ width: (p.width / Math.max(p.width, p.height)) * 24, height: (p.height / Math.max(p.width, p.height)) * 24 }}
                  />
                </span>
                <span className="flex-1">{p.label}</span>
                <span className="text-[11px] tabular-nums text-(--st-muted)">
                  {p.width}×{p.height}
                </span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
