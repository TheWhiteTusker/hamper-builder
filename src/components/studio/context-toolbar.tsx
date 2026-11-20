"use client";

import { Blend, Copy, Lock, LockOpen, Trash2 } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "./editor";
import { Popover, Slider, ToolButton } from "./studio-ui";
import { AlignMenu, PositionMenu } from "./toolbar/arrange-menus";
import { ImageTools } from "./toolbar/image-tools";
import { LineTools } from "./toolbar/line-tools";
import { PageTools } from "./toolbar/page-tools";
import { ShapeTools } from "./toolbar/shape-tools";
import { TextTools } from "./toolbar/text-tools";

const icon = "h-4 w-4";

export function ContextToolbar({ ed }: { ed: Editor }) {
  const l = ed.selected;

  return (
    <div className="flex min-h-12 flex-wrap items-center gap-1 border-b border-(--st-line) bg-(--st-panel) px-3 py-1.5">
      {ed.selection.length > 1 ? <MultiTools ed={ed} /> : !l ? <PageTools ed={ed} /> : <LayerTools ed={ed} l={l} />}
    </div>
  );
}

function LayerTools({ ed, l }: { ed: Editor; l: Layer }) {
  const locked = l.locked;

  return (
    <>
      {!locked && l.kind === "text" && <TextTools ed={ed} l={l} />}
      {!locked && (l.kind === "rect" || l.kind === "ellipse" || l.kind === "polygon" || l.kind === "star") && (
        <ShapeTools ed={ed} l={l} />
      )}
      {!locked && (l.kind === "line" || l.kind === "curve") && <LineTools ed={ed} l={l} />}
      {!locked && l.kind === "image" && <ImageTools ed={ed} l={l} />}
      {locked && <span className="text-[12px] text-(--st-muted)">This element is locked.</span>}

      <div className="ml-auto flex items-center gap-1">
        {!locked && <PositionMenu ed={ed} ids={[l.id]} />}
        <Popover title="Transparency" align="right" width={220} trigger={<Blend className={icon} />}>
          <Slider
            label="Transparency"
            value={Math.round(l.opacity * 100)}
            min={0}
            max={100}
            suffix="%"
            onChange={(v) => ed.tweak(l.id, { opacity: v / 100 })}
          />
        </Popover>
        <ToolButton title={locked ? "Unlock" : "Lock"} active={locked} onClick={() => ed.patch(l.id, { locked: !locked })}>
          {locked ? <Lock className={icon} /> : <LockOpen className={icon} />}
        </ToolButton>
        <ToolButton title="Duplicate (Ctrl+D)" onClick={() => ed.duplicate([l.id])}>
          <Copy className={icon} />
        </ToolButton>
        <ToolButton title="Delete (Del)" onClick={() => ed.remove([l.id])}>
          <Trash2 className={icon} />
        </ToolButton>
      </div>
    </>
  );
}

function MultiTools({ ed }: { ed: Editor }) {
  const ids = ed.selectedIds;
  const n = ed.selection.length;
  const allLocked = ed.selection.every((l) => l.locked);
  const opacity = Math.round((ed.selection[0]?.opacity ?? 1) * 100);

  return (
    <>
      <span className="mr-2 rounded-md bg-(--st-accent-soft) px-2 py-1 text-[12px]">{n} elements selected</span>
      <AlignMenu ed={ed} ids={ids} />
      <button type="button" className="ml-1 text-[12px] text-(--st-muted) underline hover:text-(--st-text)" onClick={() => ed.select(null)}>
        Clear selection
      </button>

      <div className="ml-auto flex items-center gap-1">
        <PositionMenu ed={ed} ids={ids} />
        <Popover title="Transparency" align="right" width={220} trigger={<Blend className={icon} />}>
          <Slider
            label="Transparency (all)"
            value={opacity}
            min={0}
            max={100}
            suffix="%"
            onChange={(v) => ed.patchEach(ids, { opacity: v / 100 }, `opacity:${ids.join()}`)}
          />
        </Popover>
        <ToolButton title={allLocked ? "Unlock all" : "Lock all"} active={allLocked} onClick={() => ed.patchEach(ids, { locked: !allLocked })}>
          {allLocked ? <Lock className={icon} /> : <LockOpen className={icon} />}
        </ToolButton>
        <ToolButton title="Duplicate all (Ctrl+D)" onClick={() => ed.duplicate(ids)}>
          <Copy className={icon} />
        </ToolButton>
        <ToolButton title="Delete all (Del)" onClick={() => ed.remove(ids)}>
          <Trash2 className={icon} />
        </ToolButton>
      </div>
    </>
  );
}
