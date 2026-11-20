"use client";

import { AlignCenter, AlignLeft, AlignRight, Bold, Italic, Minus, Plus, Underline } from "lucide-react";
import type { Layer } from "@/lib/hamper-canvas";
import type { Editor } from "../editor";
import { ColorButton, Divider, FontPicker, Popover, Slider, ToolButton } from "../studio-ui";

const icon = "h-4 w-4";

export function TextTools({ ed, l }: { ed: Editor; l: Extract<Layer, { kind: "text" }> }) {
  const bold = l.fontStyle.includes("bold");
  const italic = l.fontStyle.includes("italic");
  const style = (b: boolean, i: boolean) => (b && i ? "italic bold" : b ? "bold" : i ? "italic" : "normal");
  const size = Math.round(l.fontSize * Math.abs(l.scaleY));
  // Shown as it appears on the page, so any corner-drag scale is included.
  const setSize = (v: number) => ed.tweak(l.id, { fontSize: Math.max(6, Math.min(800, v)) / Math.abs(l.scaleY) });
  const nextAlign = { left: "center", center: "right", right: "left" } as const;
  const AlignIcon = { left: AlignLeft, center: AlignCenter, right: AlignRight }[l.align];

  return (
    <>
      <FontPicker value={l.fontFamily} onChange={(fontFamily) => ed.patch(l.id, { fontFamily })} />
      <div className="flex items-center rounded-md border border-(--st-line)">
        <ToolButton title="Decrease font size" className="h-7 w-7 px-0" onClick={() => setSize(size - 1)}>
          <Minus className="h-3.5 w-3.5" />
        </ToolButton>
        <input
          aria-label="Font size"
          className="h-7 w-11 bg-transparent text-center tabular-nums outline-none"
          value={size}
          onChange={(e) => Number(e.target.value) > 0 && setSize(Number(e.target.value))}
        />
        <ToolButton title="Increase font size" className="h-7 w-7 px-0" onClick={() => setSize(size + 1)}>
          <Plus className="h-3.5 w-3.5" />
        </ToolButton>
      </div>
      <ColorButton
        letter
        title="Text colour"
        fill={l.fill}
        documentColors={ed.documentColors}
        onChange={(fill) => ed.tweak(l.id, { fill })}
      />
      <ToolButton title="Bold" active={bold} onClick={() => ed.patch(l.id, { fontStyle: style(!bold, italic) })}>
        <Bold className={icon} />
      </ToolButton>
      <ToolButton title="Italic" active={italic} onClick={() => ed.patch(l.id, { fontStyle: style(bold, !italic) })}>
        <Italic className={icon} />
      </ToolButton>
      <ToolButton
        title="Underline"
        active={l.textDecoration === "underline"}
        onClick={() => ed.patch(l.id, { textDecoration: l.textDecoration === "underline" ? "none" : "underline" })}
      >
        <Underline className={icon} />
      </ToolButton>
      <ToolButton title={`Alignment: ${l.align}`} onClick={() => ed.patch(l.id, { align: nextAlign[l.align] })}>
        <AlignIcon className={icon} />
      </ToolButton>
      <Popover title="Spacing" width={240} trigger={<span className="text-[12.5px]">Spacing</span>}>
        <div className="space-y-3">
          <Slider
            label="Letter spacing"
            value={l.letterSpacing}
            min={-10}
            max={60}
            onChange={(letterSpacing) => ed.tweak(l.id, { letterSpacing })}
          />
          <Slider
            label="Line spacing"
            value={l.lineHeight}
            min={0.5}
            max={3}
            step={0.05}
            onChange={(lineHeight) => ed.tweak(l.id, { lineHeight })}
          />
        </div>
      </Popover>
      <Divider />
    </>
  );
}
