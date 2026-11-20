"use client";

import { useState } from "react";
import { Check, Plus } from "lucide-react";
import { fillCss, type Fill } from "@/lib/hamper-canvas";
import { cx, fieldCls, panelTitle } from "./base";
import { GradientEditor } from "./gradient-editor";

/* ------------------------------------------------------------------ colour */

const DEFAULT_COLORS = [
  "#000000", "#545454", "#737373", "#a6a6a6", "#d9d9d9", "#ffffff",
  "#ff3131", "#ff5757", "#ff66c4", "#cb6ce6", "#8c52ff", "#5e17eb",
  "#0097b2", "#0cc0df", "#5ce1e6", "#38b6ff", "#5271ff", "#004aad",
  "#00bf63", "#7ed957", "#c1ff72", "#ffde59", "#ffbd59", "#ff914d",
];

/** latticelane.com's palette: deep green, sage, soft gold, cream, sand, bronze. */
const BRAND_COLORS = ["#2c332f", "#54655b", "#ddcf8b", "#faf8ee", "#f1ebe0", "#b08d57"];

const GRADIENT_PRESETS: Fill[] = [
  { type: "linear", from: "#faf8ee", to: "#ddcf8b", angle: 90 },
  { type: "linear", from: "#54655b", to: "#2c332f", angle: 90 },
  { type: "linear", from: "#ffde59", to: "#ff914d", angle: 45 },
  { type: "linear", from: "#ff66c4", to: "#ffde59", angle: 45 },
  { type: "linear", from: "#5de0e6", to: "#004aad", angle: 45 },
  { type: "linear", from: "#8c52ff", to: "#ff914d", angle: 0 },
  { type: "linear", from: "#ffffff", to: "#d9d9d9", angle: 90 },
  { type: "linear", from: "#000000", to: "#737373", angle: 90 },
  { type: "linear", from: "#b08d57", to: "#f1ebe0", angle: 135 },
  { type: "linear", from: "#0cc0df", to: "#ffde59", angle: 90 },
];

function Swatch({ fill, selected, onClick, title }: { fill: Fill; selected?: boolean; onClick: () => void; title: string }) {
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      onClick={onClick}
      className={cx(
        "relative aspect-square w-full rounded-md border border-black/10 transition-transform hover:scale-110",
        selected && "ring-2 ring-(--st-accent) ring-offset-2 ring-offset-(--st-panel)",
      )}
      style={{ background: fillCss(fill) }}
    >
      {selected && <Check className="absolute inset-0 m-auto h-3.5 w-3.5 text-white mix-blend-difference" />}
    </button>
  );
}

/** Rainbow "+" swatch that opens the system colour picker. */
function CustomColor({ value, onChange, title }: { value: string; onChange: (c: string) => void; title: string }) {
  return (
    <label
      title={title}
      className="relative flex aspect-square w-full cursor-pointer items-center justify-center rounded-md border border-black/10 hover:scale-110"
      style={{ background: "conic-gradient(red, yellow, lime, cyan, blue, magenta, red)" }}
    >
      <Plus className="h-4 w-4 text-white drop-shadow" />
      <input type="color" value={value} onChange={(e) => onChange(e.target.value)} className="absolute inset-0 cursor-pointer opacity-0" />
    </label>
  );
}

export function ColorPanel({
  fill,
  onChange,
  documentColors,
  allowGradient = true,
}: {
  fill: Fill;
  onChange: (fill: Fill) => void;
  documentColors: string[];
  allowGradient?: boolean;
}) {
  const [tab, setTab] = useState<"solid" | "linear">(fill.type);
  const first = fill.type === "solid" ? fill.color : fill.from;

  return (
    <div className="space-y-3">
      {allowGradient && (
        <div className="grid grid-cols-2 gap-1 rounded-md bg-(--st-panel-2) p-1">
          {(["solid", "linear"] as const).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => setTab(t)}
              className={cx("h-7 rounded text-[12px]", tab === t ? "bg-(--st-panel) text-(--st-text) shadow-sm" : "text-(--st-muted)")}
            >
              {t === "solid" ? "Solid colour" : "Gradient"}
            </button>
          ))}
        </div>
      )}

      {tab === "solid" ? (
        <>
          {documentColors.length > 0 && (
            <div>
              <div className={cx(panelTitle, "mb-1.5")}>Document colours</div>
              <div className="grid grid-cols-6 gap-1.5">
                <CustomColor value={first} title="Custom colour" onChange={(color) => onChange({ type: "solid", color })} />
                {documentColors.slice(0, 11).map((c) => (
                  <Swatch
                    key={c}
                    title={c}
                    fill={{ type: "solid", color: c }}
                    selected={fill.type === "solid" && fill.color === c}
                    onClick={() => onChange({ type: "solid", color: c })}
                  />
                ))}
              </div>
            </div>
          )}
          <div>
            <div className={cx(panelTitle, "mb-1.5")}>Lattice Lane colours</div>
            <div className="grid grid-cols-6 gap-1.5">
              {BRAND_COLORS.map((c) => (
                <Swatch
                  key={c}
                  title={c}
                  fill={{ type: "solid", color: c }}
                  selected={fill.type === "solid" && fill.color === c}
                  onClick={() => onChange({ type: "solid", color: c })}
                />
              ))}
            </div>
          </div>
          <div>
            <div className={cx(panelTitle, "mb-1.5")}>Default colours</div>
            <div className="grid grid-cols-6 gap-1.5">
              {documentColors.length === 0 && (
                <CustomColor value={first} title="Custom colour" onChange={(color) => onChange({ type: "solid", color })} />
              )}
              {DEFAULT_COLORS.map((c) => (
                <Swatch
                  key={c}
                  title={c}
                  fill={{ type: "solid", color: c }}
                  selected={fill.type === "solid" && fill.color === c}
                  onClick={() => onChange({ type: "solid", color: c })}
                />
              ))}
            </div>
          </div>
          <input
            className={cx(fieldCls, "w-full font-mono uppercase")}
            value={first}
            aria-label="Hex colour"
            onChange={(e) => /^#[0-9a-fA-F]{6}$/.test(e.target.value) && onChange({ type: "solid", color: e.target.value.toLowerCase() })}
          />
        </>
      ) : (
        <>
          <div className="grid grid-cols-5 gap-1.5">
            {GRADIENT_PRESETS.map((g, i) => (
              <Swatch key={i} title="Gradient preset" fill={g} onClick={() => onChange(g)} />
            ))}
          </div>
          <GradientEditor fill={fill} first={first} onChange={onChange} />
        </>
      )}
    </div>
  );
}
