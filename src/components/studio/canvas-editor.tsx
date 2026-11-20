"use client";

import dynamic from "next/dynamic";

// Konva draws to a real <canvas>, so the editor only ever renders in the browser.
export const CanvasEditor = dynamic(() => import("./canvas-stage"), {
  ssr: false,
  loading: () => <p className="text-sm text-(--color-muted)">Loading editor…</p>,
});
