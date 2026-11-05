"use client";

import type { CostStageWithHierarchy } from "@/lib/types";
import type { LineState } from "./lines";
import type { CostLines } from "./use-cost-lines";

type Props = { stage: CostStageWithHierarchy; line: LineState; api: CostLines };

const byName = <T extends { name: string }>(rows: T[] | undefined, name: string | null | undefined) =>
  rows?.find((r) => r.name.toLowerCase() === (name ?? "").trim().toLowerCase());

function Pick({
  value,
  options,
  placeholder,
  disabled,
  className = "",
  onChange,
}: {
  value: string;
  options: { id: string; name: string }[];
  placeholder: string;
  disabled?: boolean;
  className?: string;
  onChange: (name: string) => void;
}) {
  // A saved choice that has since been removed from the master stays visible, flagged.
  const missing = !!value && !options.some((o) => o.name === value);
  return (
    <td className="p-2">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`select text-xs py-1 px-2 ${missing ? "border-amber-400 text-amber-800" : ""} ${className}`}
        title={missing ? "No longer in the Rates & Hierarchy Master: pick a current option" : undefined}
        disabled={disabled}
      >
        <option value="">{placeholder}</option>
        {missing && <option value={value}>{value} (not in master)</option>}
        {options.map((o) => (
          <option key={o.id} value={o.name}>
            {o.name}
          </option>
        ))}
      </select>
    </td>
  );
}

/** Category -> Subcategory -> Variety picked from the Rates & Hierarchy Master. */
export function PickCells({ stage, line, api }: Props) {
  const cat = stage.categories.find((c) => c.name === line.category_name);
  const sub = cat?.subcategories.find((s) => s.name === line.subcategory_name);
  return (
    <>
      <Pick
        value={line.category_name}
        options={stage.categories}
        placeholder="Select category…"
        onChange={(name) => api.setCategory(line.tempKey, name)}
      />
      <Pick
        value={line.subcategory_name ?? ""}
        options={cat?.subcategories ?? []}
        placeholder="Select…"
        disabled={!line.category_name}
        className="font-medium"
        onChange={(name) => api.setSubcategory(line.tempKey, name)}
      />
      <Pick
        value={line.variety_name ?? ""}
        options={sub?.varieties ?? []}
        placeholder="Select…"
        disabled={!line.subcategory_name}
        className="font-semibold text-(--color-brand-dark)"
        onChange={(name) => api.setVariety(line, name)}
      />
    </>
  );
}

/**
 * Category -> Subcategory -> Variety typed in (outsourced products). Existing
 * names are suggested; new ones are added to the master on save (see
 * linkBoughtOutLines).
 */
export function TypedCells({ stage, line, api }: Props) {
  const cat = byName(stage.categories, line.category_name);
  const sub = byName(cat?.subcategories, line.subcategory_name);

  function set(field: "category_name" | "subcategory_name" | "variety_name", value: string) {
    const next = { ...line, [field]: value };
    const nextSub = byName(byName(stage.categories, next.category_name)?.subcategories, next.subcategory_name);
    const v = byName(nextSub?.varieties, next.variety_name);
    api.update(line.tempKey, {
      [field]: value,
      item_name: [next.subcategory_name, next.variety_name].filter(Boolean).join(" "),
      cost_variety_id: v?.id ?? null,
      // Picking an existing variety brings its master rate and unit.
      ...(v && field === "variety_name" ? { rate: Number(v.default_rate), unit: v.unit } : {}),
    });
  }

  const cells = [
    ["category_name", "Category", stage.categories, "e.g. Drinkware"],
    ["subcategory_name", "Subcategory", cat?.subcategories, "e.g. Steel Bottle"],
    ["variety_name", "Variety", sub?.varieties, "e.g. 500ml"],
  ] as const;
  return cells.map(([field, label, options, placeholder]) => {
    const listId = `${line.tempKey}-${field}`;
    return (
      <td key={field} className="p-2">
        <input
          value={line[field] ?? ""}
          onChange={(e) => set(field, e.target.value)}
          list={listId}
          placeholder={placeholder}
          aria-label={label}
          className="input text-xs py-1 px-2"
        />
        <datalist id={listId}>
          {options?.map((o) => <option key={o.id} value={o.name} />)}
        </datalist>
      </td>
    );
  });
}
