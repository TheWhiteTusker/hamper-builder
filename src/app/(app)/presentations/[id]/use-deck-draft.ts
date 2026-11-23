import { useState } from "react";
import type { DraftSlide } from "./draft";
import type { SlideSummary } from "./page";

export const idsOf = (list: { id: string }[]) => list.map((s) => s.id).join();

/**
 * The deck as edited on the page: nothing is written until Save. Someone
 * else's save arrives as a refresh and is taken unless there are unsaved
 * changes here (the save's conflict check then protects those).
 */
export function useDeckDraft(deck: { title: string }, initialSlides: SlideSummary[]) {
  const [slides, setSlides] = useState<DraftSlide[]>(initialSlides);
  const [title, setTitle] = useState(deck.title);
  const [saved, setSaved] = useState({ title: deck.title, ids: idsOf(initialSlides) });
  const dirty = title.trim() !== saved.title || idsOf(slides) !== saved.ids;

  const [shown, setShown] = useState(initialSlides);
  if (initialSlides !== shown) {
    setShown(initialSlides);
    if (!dirty) {
      setSlides(initialSlides);
      setTitle(deck.title);
      setSaved({ title: deck.title, ids: idsOf(initialSlides) });
    }
  }

  return { slides, setSlides, title, setTitle, saved, setSaved, dirty };
}
