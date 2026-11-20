"use client";

import { Package, Palette, Shapes, Type, Upload, X } from "lucide-react";
import type { Editor, PickerProduct } from "./editor";
import { cx, toolBtn } from "./studio-ui";
import { BackgroundPanel } from "./panels/background-panel";
import { ElementsPanel } from "./panels/elements-panel";
import { ProductsPanel } from "./panels/products-panel";
import { TextPanel } from "./panels/text-panel";
import { UploadsPanel } from "./panels/uploads-panel";

export type SideTab = "products" | "uploads" | "text" | "elements" | "background";

const RAIL: { id: SideTab; label: string; Icon: typeof Package }[] = [
  { id: "products", label: "Products", Icon: Package },
  { id: "uploads", label: "Uploads", Icon: Upload },
  { id: "text", label: "Text", Icon: Type },
  { id: "elements", label: "Elements", Icon: Shapes },
  { id: "background", label: "Background", Icon: Palette },
];

export function Rail({ tab, onTab }: { tab: SideTab | null; onTab: (t: SideTab | null) => void }) {
  return (
    <nav className="flex w-18 shrink-0 flex-col items-center gap-1 border-r border-(--st-line) bg-(--st-panel) py-2">
      {RAIL.map(({ id, label, Icon }) => (
        <button
          key={id}
          type="button"
          onClick={() => onTab(tab === id ? null : id)}
          className={cx(
            "flex w-15.5 flex-col items-center gap-1 rounded-lg py-2 text-[10.5px] transition-colors",
            tab === id ? "bg-(--st-panel-2) text-(--st-text)" : "text-(--st-muted) hover:text-(--st-text)",
          )}
        >
          <Icon className={cx("h-5 w-5", tab === id && "text-(--st-accent)")} strokeWidth={1.8} />
          {label}
        </button>
      ))}
    </nav>
  );
}

export function SidePanel({
  tab,
  onClose,
  ed,
  products,
  hamperProductIds,
}: {
  tab: SideTab;
  onClose: () => void;
  ed: Editor;
  products: PickerProduct[];
  hamperProductIds: string[];
}) {
  return (
    <aside className="flex w-75 shrink-0 flex-col border-r border-(--st-line) bg-(--st-panel)">
      <div className="flex h-11 items-center justify-between px-4">
        <span className="text-[14px] font-semibold">{RAIL.find((r) => r.id === tab)?.label}</span>
        <button type="button" onClick={onClose} className={toolBtn} aria-label="Close panel" title="Close panel">
          <X className="h-4 w-4" />
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        {tab === "products" && <ProductsPanel ed={ed} products={products} hamperProductIds={hamperProductIds} />}
        {tab === "uploads" && <UploadsPanel ed={ed} />}
        {tab === "text" && <TextPanel ed={ed} />}
        {tab === "elements" && <ElementsPanel ed={ed} />}
        {tab === "background" && <BackgroundPanel ed={ed} />}
      </div>
    </aside>
  );
}
