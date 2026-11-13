"use client";

/** A select that adds the picked hamper or product, then resets to its placeholder. */
export function ItemPicker({
  id,
  label,
  placeholder,
  items,
  added,
  onPick,
}: {
  id: string;
  label: string;
  placeholder: string;
  items: { id: string; code: string; name: string }[];
  added: Set<string>;
  onPick: (id: string) => void;
}) {
  return (
    <div>
      <label className="label" htmlFor={id}>
        {label}
      </label>
      <select id={id} className="select mt-1 min-w-[220px]" value="" onChange={(e) => e.target.value && onPick(e.target.value)}>
        <option value="">{placeholder}</option>
        {items.map((h) => (
          <option key={h.id} value={h.id}>
            {h.code} — {h.name}
            {added.has(h.code) ? " (already added)" : ""}
          </option>
        ))}
      </select>
    </div>
  );
}
