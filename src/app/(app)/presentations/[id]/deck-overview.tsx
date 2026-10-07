"use client";

import { useEffect, useState, useTransition } from "react";
import Link from "@/components/link";
import { PageHeader } from "@/components/ui";
import { exportPptx } from "@/components/studio/export-pptx";
import { blankSlide } from "@/lib/presentation";
import { deletePresentation, saveDeck } from "../deck-actions";
import type { DeckSlideInput } from "../deck-schemas";
import type { PickerData } from "../item-picker";
import type { SlideSummary } from "./page";
import { AddSlides } from "./add-slides";
import { draftOf, insertBeforeClosing, type DraftSlide } from "./draft";
import { SlideCard } from "./slide-card";

export function DeckOverview({
  deck,
  slides: initialSlides,
  picker,
}: {
  deck: { id: string; title: string };
  slides: SlideSummary[];
  picker: PickerData;
}) {
  // Everything below is a draft kept on this page. Nothing is written until Save.
  const [slides, setSlides] = useState<DraftSlide[]>(initialSlides);
  const [title, setTitle] = useState(deck.title);
  const [saved, setSaved] = useState({ title: deck.title, ids: initialSlides.map((s) => s.id).join() });
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [exporting, setExporting] = useState<string | null>(null);
  const [saving, startSaving] = useTransition();
  const [deleting, startDelete] = useTransition();

  const dirty = title.trim() !== saved.title || slides.map((s) => s.id).join() !== saved.ids;

  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  const edit = (fn: (list: DraftSlide[]) => DraftSlide[]) => {
    setSlides(fn);
    setError(null);
    setNotice(null);
  };

  const move = (id: string, by: -1 | 1) =>
    edit((list) => {
      const i = list.findIndex((s) => s.id === id);
      const j = i + by;
      if (i < 0 || j < 0 || j >= list.length) return list;
      const next = [...list];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });

  const remove = (id: string) => edit((list) => list.filter((s) => s.id !== id));

  const addBlank = () =>
    edit((list) => insertBeforeClosing(list, [draftOf({ kind: "blank", hamper_id: null, product_id: null, canvas: blankSlide() })]));


  const save = () =>
    startSaving(async () => {
      setError(null);
      setNotice(null);
      const payload: DeckSlideInput[] = slides.map((s) => (s.unsaved ? { ...s.unsaved, tempId: s.id } : { id: s.id }));
      const res = await saveDeck(deck.id, { title: title.trim(), slides: payload });
      if (res.error) {
        setError(res.error);
        return;
      }
      // Swap temporary ids for the real ones so the new slides can be opened.
      const next = slides.map((s) => (s.unsaved && res.ids?.[s.id] ? { id: res.ids[s.id], kind: s.kind, canvas: s.canvas } : s));
      setSlides(next);
      setSaved({ title: title.trim(), ids: next.map((s) => s.id).join() });
      setNotice("Saved.");
    });

  const discard = () => {
    if (!confirm("Discard your unsaved changes?")) return;
    setSlides(initialSlides);
    setTitle(saved.title);
    // initialSlides is what the page loaded with; after an earlier save, reload to get the saved state.
    if (initialSlides.map((s) => s.id).join() !== saved.ids) window.location.reload();
  };

  /** Leaving for a slide editor would lose the draft; ask first. */
  const guard = (e: React.MouseEvent) => {
    if (dirty && !confirm("You have unsaved changes to this presentation. Leave without saving?")) e.preventDefault();
  };

  async function downloadPptx() {
    setError(null);
    try {
      await exportPptx(title.trim() || saved.title, slides.map((s) => s.canvas), (done) =>
        setExporting(done < slides.length ? `Building slide ${done + 1} of ${slides.length}…` : "Saving file…"),
      );
    } catch {
      setError("Could not build the PowerPoint. An image from another website may be blocking the export.");
    } finally {
      setExporting(null);
    }
  }

  return (
    <>
      <PageHeader title={saved.title} subtitle={`${slides.length} slides${dirty ? " · unsaved changes" : ""}`}>
        {dirty && (
          <button type="button" className="btn-secondary" onClick={discard} disabled={saving}>
            Discard
          </button>
        )}
        <button type="button" className="btn-primary" onClick={save} disabled={!dirty || saving || !title.trim()}>
          {saving ? "Saving…" : dirty ? "Save changes" : "Saved"}
        </button>
        {dirty ? (
          <span className="btn-secondary cursor-not-allowed opacity-50" title="Save your changes first">
            Download PDF
          </span>
        ) : (
          <Link href={`/presentations/${deck.id}/print`} prefetch={false} target="_blank" className="btn-secondary">
            Download PDF
          </Link>
        )}
        <button type="button" className="btn-secondary" onClick={downloadPptx} disabled={!!exporting || !slides.length}>
          {exporting ?? "Download PowerPoint"}
        </button>
      </PageHeader>

      {error && <p className="mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">{error}</p>}
      {notice && !dirty && (
        <p className="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">{notice}</p>
      )}

      <div className="card mb-4 flex flex-wrap items-end gap-3 p-3">
        <div className="min-w-64 flex-1">
          <label className="label" htmlFor="deck-title">
            Presentation name
          </label>
          <input
            id="deck-title"
            className="input mt-1"
            value={title}
            onChange={(e) => {
              setTitle(e.target.value);
              setNotice(null);
            }}
          />
        </div>
        <button
          type="button"
          className="btn-danger ml-auto"
          disabled={deleting}
          onClick={() =>
            confirm(`Delete "${saved.title}" and all its slides?`) &&
            startDelete(async () => {
              const res = await deletePresentation(deck.id);
              if (res?.error) setError(res.error);
            })
          }
        >
          {deleting ? "Deleting…" : "Delete presentation"}
        </button>
      </div>

      <p className="mb-3 text-sm text-[var(--color-muted)]">
        Add, remove and reorder slides, then click <strong>Save changes</strong>. Click a slide to edit it in the photo editor.
      </p>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
        {slides.map((s, i) => (
          <SlideCard
            key={s.id}
            s={s}
            i={i}
            last={i === slides.length - 1}
            deckId={deck.id}
            guard={guard}
            onMove={(by) => move(s.id, by)}
            onRemove={() => remove(s.id)}
          />
        ))}
      </div>

      <AddSlides
        picker={picker}
        onAdd={(built) => edit((list) => insertBeforeClosing(list, built.map(draftOf)))}
        onBlank={addBlank}
        onError={setError}
      />
    </>
  );
}
