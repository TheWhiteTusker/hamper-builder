"use client";

import { useEffect, useRef, useState } from "react";
import Link from "@/components/link";
import type { HamperCanvas } from "@/lib/hamper-canvas";
import { PX_PER_INCH } from "@/lib/presentation";
import { renderCanvas } from "@/components/studio/render";

/**
 * Renders every slide to a full-size picture, then opens the browser's print
 * dialog; "Save as PDF" there gives one page per slide at the slide's shape.
 */
export function PrintDeck({ deck, slides }: { deck: { id: string; title: string }; slides: { id: string; canvas: HamperCanvas }[] }) {
  const [images, setImages] = useState<(string | null)[]>(() => slides.map(() => null));
  const [failed, setFailed] = useState(false);
  const printed = useRef(false);
  const done = images.filter(Boolean).length;
  const ready = slides.length > 0 && done === slides.length;
  const first = slides[0]?.canvas;

  useEffect(() => {
    let live = true;
    (async () => {
      for (const [i, s] of slides.entries()) {
        try {
          const { dataUrl } = await renderCanvas(s.canvas);
          if (!live) return;
          setImages((prev) => prev.map((v, j) => (j === i ? dataUrl : v)));
        } catch {
          if (live) setFailed(true);
          return;
        }
      }
    })();
    return () => {
      live = false;
    };
  }, [slides]);

  useEffect(() => {
    if (!ready || printed.current) return;
    printed.current = true;
    document.title = deck.title; // becomes the suggested PDF file name
    setTimeout(() => window.print(), 300);
  }, [ready, deck.title]);

  return (
    <>
      {first && (
        // One page per slide, exactly the slide's shape, no margins; the app chrome is hidden.
        <style>{`
          @media print {
            @page { size: ${first.width / PX_PER_INCH}in ${first.height / PX_PER_INCH}in; margin: 0; }
            main { padding: 0 !important; max-width: none !important; }
            .deck-page { width: 100vw; height: 100vh; margin: 0 !important; box-shadow: none !important; break-after: page; }
            .deck-page:last-child { break-after: auto; }
          }
        `}</style>
      )}

      <div className="no-print mb-4 flex flex-wrap items-center gap-3">
        <Link href={`/presentations/${deck.id}`} className="btn-secondary">
          Back
        </Link>
        <span className="text-sm text-[var(--color-muted)]">
          {failed
            ? "A slide could not be drawn. An image from another website may be blocking it."
            : ready
              ? `${slides.length} slides ready. In the print dialog choose "Save as PDF".`
              : `Preparing slide ${Math.min(done + 1, slides.length)} of ${slides.length}…`}
        </span>
        <button type="button" className="btn-primary ml-auto" disabled={!ready} onClick={() => window.print()}>
          Print / Save as PDF
        </button>
      </div>

      <div className="space-y-6">
        {slides.map((s, i) => (
          <div
            key={s.id}
            className="deck-page mx-auto flex max-w-5xl items-center justify-center overflow-hidden bg-white shadow-md"
            style={{ aspectRatio: `${s.canvas.width} / ${s.canvas.height}` }}
          >
            {images[i] ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={images[i]!} alt={`Slide ${i + 1}`} className="h-full w-full object-contain" />
            ) : (
              <span className="text-sm text-[var(--color-muted)]">Slide {i + 1}…</span>
            )}
          </div>
        ))}
      </div>
    </>
  );
}
