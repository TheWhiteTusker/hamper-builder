"use client";

import { fillCss, type Layer } from "@/lib/hamper-canvas";
import { cx } from "../studio-ui";

/** A small preview of a layer in the Layers panel. */
export function Thumb({ l }: { l: Layer }) {
  const box = "flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded border border-(--st-line)";
  if (l.kind === "image") {
    return (
      <span className={cx(box, "checkerboard")}>
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img src={l.url} alt="" className="h-full w-full object-contain" />
      </span>
    );
  }
  if (l.kind === "text") {
    return (
      <span className={cx(box, "bg-(--st-panel-2) text-[15px] font-semibold")} style={{ fontFamily: l.fontFamily }}>
        T
      </span>
    );
  }
  if (l.kind === "line" || l.kind === "curve") {
    return (
      <span className={cx(box, "bg-(--st-panel-2)")}>
        <svg viewBox="0 0 24 24" className="h-5 w-5" stroke={l.stroke} strokeWidth="2.5" fill="none" strokeLinecap="round">
          {l.kind === "line" ? (
            <line x1="4" y1="20" x2="20" y2="4" strokeDasharray={l.dash?.length ? "4 3" : undefined} />
          ) : (
            <path d="M4 18 C8 6, 16 6, 20 18" strokeDasharray={l.dash?.length ? "4 3" : undefined} />
          )}
        </svg>
      </span>
    );
  }
  return (
    <span className={cx(box, "bg-(--st-panel-2)")}>
      <span
        className={cx("h-5 w-5", l.kind === "ellipse" && "rounded-full", l.kind === "rect" && l.cornerRadius > 0 && "rounded")}
        style={{
          background: fillCss(l.fill),
          clipPath:
            l.kind === "polygon" && l.sides === 3
              ? "polygon(50% 0, 100% 100%, 0 100%)"
              : l.kind === "star"
                ? "polygon(50% 0,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)"
                : l.kind === "polygon"
                  ? "polygon(50% 0,100% 25%,100% 75%,50% 100%,0 75%,0 25%)"
                  : undefined,
        }}
      />
    </span>
  );
}
