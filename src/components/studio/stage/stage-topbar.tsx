"use client";

import Link from "@/components/link";
import { ArrowLeft, Check, Download, Loader2, Redo2, Scaling, Undo2 } from "lucide-react";
import { redo, undo } from "@/lib/hamper-canvas";
import { Popover, ToolButton, accentBtn, cx, toolBtn } from "../studio-ui";
import { ResizeMenu } from "./resize-menu";
import type { CanvasDoc } from "./use-canvas-doc";

export function StageTopbar({
  title,
  subtitle,
  backHref,
  doc,
  saving,
  onResize,
  onDownload,
  onSave,
}: {
  title: string;
  subtitle?: string;
  backHref: string;
  doc: CanvasDoc;
  saving: boolean;
  onResize: (w: number, h: number) => void;
  onDownload: () => void;
  onSave: () => void;
}) {
  const { hist, setHist, dirty, canvas } = doc;
  return (
    <header className="studio-topbar flex h-12 shrink-0 items-center gap-1 px-2">
      <Link
        href={backHref}
        className={toolBtn}
        onClick={(e) => {
          if (dirty && !confirm("You have unsaved changes. Leave without saving?")) e.preventDefault();
        }}
      >
        <ArrowLeft className="h-4 w-4" /> Back
      </Link>
      <span className="mx-2 h-5 w-px bg-[var(--st-line)]" />
      <div className="mr-3 min-w-0">
        <div className="truncate text-[13px] font-semibold leading-tight">{title}</div>
        {subtitle && <div className="text-[11px] leading-tight text-[var(--st-muted)]">{subtitle}</div>}
      </div>
      <ToolButton title="Undo (Ctrl+Z)" disabled={!hist.past.length} onClick={() => setHist(undo)}>
        <Undo2 className="h-4 w-4" />
      </ToolButton>
      <ToolButton title="Redo (Ctrl+Shift+Z)" disabled={!hist.future.length} onClick={() => setHist(redo)}>
        <Redo2 className="h-4 w-4" />
      </ToolButton>
      <Popover title="Resize" width={280} trigger={<><Scaling className="h-4 w-4" /> Resize</>}>
        {(close) => (
          <ResizeMenu
            width={canvas.width}
            height={canvas.height}
            onResize={(w, h) => {
              onResize(w, h);
              close();
            }}
          />
        )}
      </Popover>

      <span className="ml-auto mr-2 flex items-center gap-1.5 text-[12px] text-[var(--st-muted)]">
        {saving ? (
          <>
            <Loader2 className="h-3.5 w-3.5 animate-spin" /> Saving…
          </>
        ) : dirty ? (
          "Unsaved changes"
        ) : (
          <>
            <Check className="h-3.5 w-3.5" /> All changes saved
          </>
        )}
      </span>
      <ToolButton title="Download PNG" onClick={onDownload}>
        <Download className="h-4 w-4" /> Download
      </ToolButton>
      <button type="button" className={cx(accentBtn, "ml-1")} onClick={onSave} disabled={saving} title="Save (Ctrl+S)">
        {saving ? "Saving…" : "Save"}
      </button>
    </header>
  );
}
