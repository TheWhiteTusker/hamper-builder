"use client";

import { useLayoutEffect, useRef, useState } from "react";
import type { Box, Layer } from "@/lib/hamper-canvas";
import { cx } from "../studio-ui";
import type { CanvasDoc } from "./use-canvas-doc";
import type { Toast } from "./use-stage-save";
import type { StageView } from "./use-stage-view";

type TextLayer = Extract<Layer, { kind: "text" }>;

/** Inline text editing, positioned over the text it replaces. */
function TextEditor({ layer: t, view, doc }: { layer: TextLayer; view: StageView; doc: CanvasDoc }) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const { ox, oy, zoom } = view;

  // Size the editor to its content.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "0px";
    el.style.height = `${el.scrollHeight}px`;
  }, [t.text, t.fontSize, t.width, zoom, t.id]);

  return (
    <textarea
      ref={ref}
      autoFocus
      value={t.text}
      spellCheck={false}
      onFocus={(e) => e.target.select()}
      onChange={(e) => doc.tweak(t.id, { text: e.target.value })}
      onBlur={() => doc.setEditingId(null)}
      onKeyDown={(e) => e.key === "Escape" && e.currentTarget.blur()}
      className="absolute resize-none overflow-hidden border-0 bg-transparent p-0 outline outline-2 outline-(--st-accent)"
      style={{
        left: ox + t.x * zoom,
        top: oy + t.y * zoom,
        width: t.width * Math.abs(t.scaleX) * zoom,
        fontSize: t.fontSize * Math.abs(t.scaleY) * zoom,
        fontFamily: t.fontFamily,
        fontWeight: t.fontStyle.includes("bold") ? 700 : 400,
        fontStyle: t.fontStyle.includes("italic") ? "italic" : "normal",
        textAlign: t.align,
        lineHeight: t.lineHeight,
        letterSpacing: t.letterSpacing * Math.abs(t.scaleX) * zoom,
        color: t.fill.type === "solid" ? t.fill.color : t.fill.from,
        opacity: t.opacity,
        transform: `rotate(${t.rotation}deg)`,
        transformOrigin: "top left",
        whiteSpace: "pre-wrap",
        overflowWrap: "break-word",
      }}
    />
  );
}

/** Handles on the page's right and bottom edges; dragging resizes the page. */
function PageResizeHandles({ view, doc }: { view: StageView; doc: CanvasDoc }) {
  const [resizing, setResizing] = useState(false);
  const { ox, oy, zoom } = view;
  const W = doc.canvas.width;
  const H = doc.canvas.height;

  // The zoom and top-left corner stay put while dragging.
  function start(e: React.PointerEvent<HTMLDivElement>, edge: "e" | "s" | "se") {
    e.preventDefault();
    e.stopPropagation();
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const z = zoom;
    const from = { x: e.clientX, y: e.clientY, w: W, h: H, panX: view.view.panX, panY: view.view.panY };
    const size = (v: number) => Math.round(Math.min(8000, Math.max(100, v)));
    setResizing(true);
    const move = (ev: PointerEvent) => {
      const w = edge === "s" ? from.w : size(from.w + (ev.clientX - from.x) / z);
      // Shift on the corner keeps the page's proportions.
      const h =
        edge === "e"
          ? from.h
          : edge === "se" && ev.shiftKey
            ? size((w * from.h) / from.w)
            : size(from.h + (ev.clientY - from.y) / z);
      doc.change((c) => (c.width === w && c.height === h ? c : { ...c, width: w, height: h }), "page-size");
      view.setView({ zoom: z, panX: from.panX + ((w - from.w) * z) / 2, panY: from.panY + ((h - from.h) * z) / 2 });
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      setResizing(false);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  }

  return (
    <>
      {(["e", "s", "se"] as const).map((edge) => (
        <div
          key={edge}
          title="Drag to resize the page (Shift on the corner keeps proportions)"
          onPointerDown={(e) => start(e, edge)}
          className={cx(
            "absolute z-10 -translate-x-1/2 -translate-y-1/2 border-2 border-(--st-accent) bg-(--st-panel) shadow-sm hover:bg-(--st-gold)",
            edge === "se" && "h-4 w-4 cursor-nwse-resize rounded-full",
            edge === "e" && "h-9 w-2.5 cursor-ew-resize rounded-full",
            edge === "s" && "h-2.5 w-9 cursor-ns-resize rounded-full",
          )}
          style={{
            left: edge === "s" ? ox + (W * zoom) / 2 : ox + W * zoom,
            top: edge === "e" ? oy + (H * zoom) / 2 : oy + H * zoom,
          }}
        />
      ))}
      {resizing && (
        <div
          className="pointer-events-none absolute z-10 rounded-md bg-(--st-accent-strong) px-2 py-1 text-[12px] tabular-nums text-(--st-on-accent) shadow"
          style={{ left: ox + W * zoom + 14, top: oy + H * zoom + 14 }}
        >
          {W} × {H} px
        </div>
      )}
    </>
  );
}

export function BoardOverlays({
  doc,
  view,
  marquee,
  swapping,
  toast,
}: {
  doc: CanvasDoc;
  view: StageView;
  marquee: Box | null;
  swapping: boolean;
  toast: Toast;
}) {
  const { editing } = doc;
  return (
    <>
      {editing && <TextEditor layer={editing} view={view} doc={doc} />}
      {view.board.w > 0 && !editing && <PageResizeHandles view={view} doc={doc} />}

      {marquee && (
        <div
          className="pointer-events-none absolute rounded-sm border border-(--st-accent) bg-(--st-accent-soft)"
          style={{ left: marquee.x, top: marquee.y, width: marquee.width, height: marquee.height }}
        />
      )}

      {swapping && (
        <div className="pointer-events-none absolute left-1/2 top-3 -translate-x-1/2 rounded-full bg-(--st-accent) px-4 py-1.5 text-[12px] font-medium text-(--st-on-accent) shadow-lg">
          Choose a new image from the Products panel
        </div>
      )}

      {toast && (
        <div
          role="status"
          className={cx(
            "absolute bottom-4 left-1/2 max-w-md -translate-x-1/2 rounded-lg px-4 py-2.5 text-[13px] shadow-2xl",
            toast.kind === "error" ? "bg-red-700 text-white" : "bg-(--st-accent-strong) text-(--st-on-accent)",
          )}
        >
          {toast.text}
        </div>
      )}
    </>
  );
}
