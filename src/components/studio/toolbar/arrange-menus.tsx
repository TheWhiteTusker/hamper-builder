"use client";

import { AlignCenterHorizontal, AlignCenterVertical, AlignEndHorizontal, AlignEndVertical, AlignStartHorizontal, AlignStartVertical, ArrowDown, ArrowUp, BringToFront, Columns3, Rows3, SendToBack } from "lucide-react";
import type { LayerMove, PageAlign } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { Popover, cx, panelTitle } from "../studio-ui";

const icon = "h-4 w-4";

/** Align several elements to each other, or space them evenly. */
export function AlignMenu({ ed, ids }: { ed: Editor; ids: string[] }) {
  const aligns: [PageAlign, string, typeof ArrowUp][] = [
    ["left", "Left", AlignStartVertical],
    ["top", "Top", AlignStartHorizontal],
    ["center", "Centre", AlignCenterVertical],
    ["middle", "Middle", AlignCenterHorizontal],
    ["right", "Right", AlignEndVertical],
    ["bottom", "Bottom", AlignEndHorizontal],
  ];
  const item = "flex items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-(--st-hover) disabled:opacity-35 disabled:hover:bg-transparent";
  return (
    <Popover title="Align elements" width={260} trigger={<><AlignCenterVertical className={icon} /> Align</>}>
      <div className={cx(panelTitle, "mb-1.5")}>Align elements</div>
      <div className="grid grid-cols-2 gap-1">
        {aligns.map(([where, label, Icon]) => (
          <button key={where} type="button" className={item} onClick={() => ed.align(ids, where)}>
            <Icon className={icon} /> {label}
          </button>
        ))}
      </div>
      <div className={cx(panelTitle, "mb-1.5 mt-3")}>Spacing</div>
      <div className="grid grid-cols-2 gap-1">
        <button type="button" className={item} disabled={ids.length < 3} onClick={() => ed.distribute(ids, "x")}>
          <Columns3 className={icon} /> Horizontally
        </button>
        <button type="button" className={item} disabled={ids.length < 3} onClick={() => ed.distribute(ids, "y")}>
          <Rows3 className={icon} /> Vertically
        </button>
      </div>
      {ids.length < 3 && <p className="mt-2 text-[11px] text-(--st-muted)">Select 3 or more to space evenly.</p>}
    </Popover>
  );
}

export function PositionMenu({ ed, ids }: { ed: Editor; ids: string[] }) {
  const arrange: [LayerMove, string, typeof ArrowUp, string][] = [
    ["forward", "Forward", ArrowUp, "Ctrl+]"],
    ["backward", "Backward", ArrowDown, "Ctrl+["],
    ["front", "To front", BringToFront, "Ctrl+Shift+]"],
    ["back", "To back", SendToBack, "Ctrl+Shift+["],
  ];
  const aligns: [PageAlign, string, typeof ArrowUp][] = [
    ["top", "Top", AlignStartHorizontal],
    ["left", "Left", AlignStartVertical],
    ["middle", "Middle", AlignCenterHorizontal],
    ["center", "Centre", AlignCenterVertical],
    ["bottom", "Bottom", AlignEndHorizontal],
    ["right", "Right", AlignEndVertical],
  ];
  const item = "flex items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-(--st-hover)";

  return (
    <Popover title="Position" align="right" width={290} trigger={<span className="text-[12.5px]">Position</span>}>
      <div className={cx(panelTitle, "mb-1.5")}>Arrange</div>
      <div className="grid grid-cols-2 gap-1">
        {arrange.map(([m, label, Icon, key]) => (
          <button key={m} type="button" className={item} onClick={() => ed.move(ids, m)} title={key}>
            <Icon className={icon} /> {label}
          </button>
        ))}
      </div>
      {ids.length === 1 && (
        <>
          <div className={cx(panelTitle, "mb-1.5 mt-3")}>Align to page</div>
          <div className="grid grid-cols-2 gap-1">
            {aligns.map(([where, label, Icon]) => (
              <button key={where} type="button" className={item} onClick={() => ed.align(ids, where)}>
                <Icon className={icon} /> {label}
              </button>
            ))}
          </div>
        </>
      )}
      <p className="mt-3 text-[11px] text-(--st-muted)">
        Tip: drag layers in the Layers panel to reorder.
      </p>
    </Popover>
  );
}
