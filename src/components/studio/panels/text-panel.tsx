"use client";

import { TEXT_COMBOS, TEXT_PRESETS, makeText, type Editor } from "../editor";
import { cx, panelTitle } from "../studio-ui";

export function TextPanel({ ed }: { ed: Editor }) {
  return (
    <div className="space-y-4">
      <p className="text-[12px] text-(--st-muted)">Click text to add it to the page. Double-click it on the page to type.</p>
      <div className="space-y-2">
        {TEXT_PRESETS.map((t) => (
          <button
            key={t.label}
            type="button"
            onClick={() => ed.add(makeText(ed.canvas, t.text, t.style))}
            className="block w-full rounded-md border border-(--st-line) bg-(--st-panel-2) px-3 py-2.5 text-left hover:border-(--st-accent)"
            style={{ fontFamily: t.style.fontFamily, fontSize: t.preview, fontWeight: t.style.fontStyle.includes("bold") ? 700 : 400 }}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div>
        <div className={cx(panelTitle, "mb-2")}>Font combinations</div>
        <div className="grid grid-cols-2 gap-2">
          {TEXT_COMBOS.map((t) => (
            <button
              key={t.text}
              type="button"
              onClick={() => ed.add(makeText(ed.canvas, t.text, t.style))}
              className="flex aspect-4/3 items-center justify-center overflow-hidden rounded-md border border-(--st-line) bg-white p-2 text-center leading-tight hover:ring-2 hover:ring-(--st-accent)"
              style={{
                fontFamily: t.style.fontFamily,
                color: t.style.color,
                fontSize: Math.min(26, t.style.fontSize / 4),
                fontWeight: t.style.fontStyle.includes("bold") ? 700 : 400,
                fontStyle: t.style.fontStyle.includes("italic") ? "italic" : "normal",
                letterSpacing: (t.style.letterSpacing ?? 0) / 4,
              }}
            >
              {t.text}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
