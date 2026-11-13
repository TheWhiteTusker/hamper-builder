"use client";

import { useMemo, useState } from "react";
import { groupByOption, num, priceOption } from "@/lib/pricing";
import type { Settings } from "@/lib/types";
import { ItemPicker } from "./item-picker";
import { LineTable } from "./line-table";
import { OptionGroup } from "./option-group";
import { nextOptionLabel, pricedLine, type HamperOption, type Line, type ProductOption } from "./quote-state";

export type AddItem = (kind: "hamper" | "product", id: string, option: string) => void;

/**
 * A combined order is one list of confirmed items with one total. An option-based
 * quote is several of those side by side (Option 1, Option 2…), each with its own
 * lines and total, for the client to choose between.
 */
export function QuoteLines({
  lines,
  structure,
  isCombined,
  hampers,
  products,
  settings,
  packagingCategories,
  canEdit,
  gstPct,
  onStructure,
  onAdd,
  onChange,
  onRemove,
  onRenameOption,
  onRemoveOption,
}: {
  lines: Line[];
  structure: string;
  isCombined: boolean;
  hampers: HamperOption[];
  products: ProductOption[];
  settings: Settings;
  packagingCategories: string[];
  canEdit: boolean;
  gstPct: string;
  onStructure: (s: string) => void;
  onAdd: AddItem;
  onChange: (key: string, patch: Partial<Line>) => void;
  onRemove: (key: string) => void;
  onRenameOption: (from: string, to: string) => void;
  onRemoveOption: (label: string) => void;
}) {
  // Options added but still empty: an option otherwise exists only through its lines.
  const [empty, setEmpty] = useState<string[]>([]);
  const groups = groupByOption(lines);
  const listed = [
    ...groups,
    ...empty.filter((e) => !groups.some((g) => g.label === e)).map((label) => ({ label, lines: [] as Line[] })),
  ];
  const options = listed.length ? listed : [{ label: "Option 1", lines: [] as Line[] }];

  const contentsOf = useMemo(() => {
    const map = new Map(hampers.map((h) => [h.id, h.items]));
    return (hamperId: string | null) => (hamperId && map.get(hamperId)) || [];
  }, [hampers]);

  const pickers = (option: string, among: Line[]) => {
    const added = new Set(among.map((l) => l.hamper_code));
    const id = option.replace(/\W/g, "-");
    return (
      <>
        <ItemPicker id={`hamper-${id}`} label="Add hamper" placeholder="Select a hamper…" items={hampers} added={added} onPick={(h) => onAdd("hamper", h, option)} />
        <ItemPicker id={`product-${id}`} label="Add product" placeholder="Select a product…" items={products} added={added} onPick={(p) => onAdd("product", p, option)} />
      </>
    );
  };
  const table = (rows: Line[]) => (
    <LineTable lines={rows} contentsOf={contentsOf} settings={settings} packagingCategories={packagingCategories} canEdit={canEdit} onChange={onChange} onRemove={onRemove} />
  );

  return (
    <section className="card overflow-hidden">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-[var(--color-line)] px-4 py-2.5">
        <div>
          <h2 className="text-sm font-semibold">Hampers & products</h2>
          <p className="text-xs text-[var(--color-muted)]">
            {isCombined
              ? "Combined order: the confirmed items, with one total."
              : "Option based: each option is its own set of items with its own total, for the client to choose from."}
          </p>
        </div>

        <div className="flex flex-wrap items-end gap-2">
          <div>
            <label className="label" htmlFor="structure">
              Structure
            </label>
            <select id="structure" className="select mt-1" value={structure} onChange={(e) => onStructure(e.target.value)} disabled={!canEdit}>
              {Array.from(new Set([...settings.quote_structures, structure])).map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </div>
          {canEdit && isCombined && pickers("Option 1", lines)}
        </div>
      </div>

      {isCombined ? (
        table(lines)
      ) : (
        <>
          {/* Keyed by position: renaming an option must not remount its section. */}
          {options.map((o, i) => (
            <OptionGroup
              key={i}
              label={o.label}
              totals={priceOption(o.lines.map(pricedLine), num(gstPct) / 100)}
              gstPct={gstPct}
              canEdit={canEdit}
              pickers={pickers(o.label, o.lines)}
              onRename={(to) => {
                // An empty option has no lines to carry the name, so it is kept here.
                setEmpty((e) => (e.includes(o.label) ? e.map((x) => (x === o.label ? to : x)) : o.lines.length ? e : [...e, to]));
                onRenameOption(o.label, to);
              }}
              onRemove={() => {
                setEmpty((e) => e.filter((x) => x !== o.label));
                onRemoveOption(o.label);
              }}
            >
              {table(o.lines)}
            </OptionGroup>
          ))}
          {canEdit && (
            <div className="border-t border-[var(--color-line)] px-4 py-3">
              <button
                type="button"
                className="btn-secondary"
                onClick={() => setEmpty((e) => [...e, ...(listed.length ? [] : ["Option 1"]), nextOptionLabel([...options.map((o) => o.label), ...e])])}
              >
                + Add option
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}
