"use client";

import type { HamperCanvas } from "@/lib/hamper-canvas";
import { SlideThumb } from "../slide-thumb";
import { cx } from "../studio-ui";

/** Sibling pages (presentation slides) as thumbnails under the page. */
export function PageStrip({
  pages,
  pageId,
  current: currentCanvas,
  dirty,
  onGo,
}: {
  pages: { id: string; href: string; canvas: HamperCanvas }[];
  pageId?: string;
  /** The last saved state of this page, drawn in its own thumbnail. */
  current: HamperCanvas;
  dirty: boolean;
  onGo: (href: string) => void;
}) {
  return (
    <div className="flex shrink-0 items-center gap-3 overflow-x-auto border-t border-(--st-line) bg-(--st-panel) px-3 py-2">
      {pages.map((p, i) => {
        const current = p.id === pageId;
        return (
          <button
            key={p.id}
            type="button"
            onClick={() => !current && onGo(p.href)}
            title={current ? "This slide" : `Go to slide ${i + 1}${dirty ? " (saves this one first)" : ""}`}
            className="group flex shrink-0 flex-col items-center gap-1"
          >
            <SlideThumb
              canvas={current ? currentCanvas : p.canvas}
              width={112}
              className={cx(
                "rounded border-2",
                current ? "border-(--st-accent)" : "border-transparent group-hover:border-(--st-muted)",
              )}
            />
            <span className={cx("text-[11px] tabular-nums", current ? "font-semibold" : "text-(--st-muted)")}>{i + 1}</span>
          </button>
        );
      })}
    </div>
  );
}
