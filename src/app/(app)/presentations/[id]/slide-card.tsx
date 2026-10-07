"use client";

import Link from "@/components/link";
import { SlideThumb } from "@/components/studio/slide-thumb";
import { KIND_LABEL, slideName, type DraftSlide } from "./draft";

/** One slide in the deck grid: thumbnail (a link once saved), name, move and remove. */
export function SlideCard({
  s,
  i,
  last,
  deckId,
  guard,
  onMove,
  onRemove,
}: {
  s: DraftSlide;
  i: number;
  last: boolean;
  deckId: string;
  /** Asks before leaving a page with unsaved changes. */
  guard: (e: React.MouseEvent) => void;
  onMove: (by: -1 | 1) => void;
  onRemove: () => void;
}) {
  return (
    <div className={`card overflow-hidden ${s.unsaved ? "ring-2 ring-[var(--color-gold)]" : ""}`}>
      {s.unsaved ? (
        <div className="relative bg-[var(--color-paper)] p-2" title="Save changes to edit this slide">
          <SlideThumb canvas={s.canvas} width={240} className="mx-auto rounded shadow-sm" />
          <span className="absolute right-3 top-3 rounded bg-[var(--color-ink)] px-1.5 py-0.5 text-[11px] text-white">New · save to edit</span>
        </div>
      ) : (
        <Link
          href={`/presentations/${deckId}/slides/${s.id}`}
          prefetch={false}
          onClick={guard}
          className="block bg-[var(--color-paper)] p-2 hover:opacity-90"
        >
          <SlideThumb canvas={s.canvas} width={240} className="mx-auto rounded shadow-sm" />
        </Link>
      )}
      <div className="flex items-center gap-2 px-3 py-2">
        <span className="w-6 text-sm tabular-nums text-[var(--color-muted)]">{i + 1}</span>
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium">{slideName(s)}</div>
          <div className="text-xs text-[var(--color-muted)]">{KIND_LABEL[s.kind] ?? s.kind}</div>
        </div>
        <button type="button" aria-label="Move earlier" className="px-1 disabled:opacity-30" disabled={i === 0} onClick={() => onMove(-1)}>
          ←
        </button>
        <button
          type="button"
          aria-label="Move later"
          className="px-1 disabled:opacity-30"
          disabled={last}
          onClick={() => onMove(1)}
        >
          →
        </button>
        <button type="button" aria-label="Remove slide" className="px-1 text-red-700" onClick={onRemove}>
          ×
        </button>
      </div>
    </div>
  );
}
