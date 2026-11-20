"use client";

import { useState } from "react";
import { DEFAULT_NOTE } from "@/lib/presentation";
import { buildSlides } from "../build-slides";
import type { DeckItem, SlideInput } from "../deck-schemas";
import { ItemPicker, type PickerData } from "../item-picker";

/** "Add hamper or product slides" and "Add blank slide". Built slides join the page's draft. */
export function AddSlides({
  picker,
  onAdd,
  onBlank,
  onError,
}: {
  picker: PickerData;
  onAdd: (slides: SlideInput[]) => void;
  onBlank: () => void;
  onError: (message: string | null) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [toAdd, setToAdd] = useState<DeckItem[]>([]);
  const [layingOut, setLayingOut] = useState(false);

  async function addItems() {
    onError(null);
    setLayingOut(true);
    try {
      // Laying out reads hamper details and photo sizes; the slides still only join the draft.
      onAdd(await buildSlides(toAdd, DEFAULT_NOTE));
      setToAdd([]);
      setAdding(false);
    } catch (e) {
      onError(e instanceof Error ? e.message : "Could not lay out the new slides.");
    } finally {
      setLayingOut(false);
    }
  }

  return (
    <section className="mt-6 space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" className="btn-secondary" onClick={() => setAdding((v) => !v)}>
          {adding ? "Close" : "Add hamper or product slides"}
        </button>
        <button type="button" className="btn-secondary" onClick={onBlank}>
          Add blank slide
        </button>
      </div>

      {adding && (
        <>
          <ItemPicker {...picker} value={toAdd} onChange={setToAdd} />
          <button type="button" className="btn-primary" disabled={layingOut || !toAdd.length} onClick={addItems}>
            {layingOut ? "Laying out slides…" : `Add ${toAdd.length} slide${toAdd.length === 1 ? "" : "s"}`}
          </button>
          <p className="text-xs text-(--color-muted)">New slides go before the ending slide. They are saved when you click Save changes.</p>
        </>
      )}
    </section>
  );
}
