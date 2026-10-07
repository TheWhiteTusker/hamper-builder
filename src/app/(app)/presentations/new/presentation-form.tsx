"use client";

import { useState, useTransition } from "react";
import Link from "@/components/link";
import { DEFAULT_NOTE } from "@/lib/presentation";
import { createPresentation } from "../deck-actions";
import type { DeckItem } from "../deck-schemas";
import { buildSlides } from "../build-slides";
import { ItemPicker, type PickerData } from "../item-picker";

export function PresentationForm({ hampers, products, defaultContact }: PickerData & { defaultContact: string }) {
  const [pending, start] = useTransition();
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState("Premium");
  const [subtitle, setSubtitle] = useState("Giveaway Hamper");
  const [closingHeading, setClosingHeading] = useState("Thank You");
  const [closingTagline, setClosingTagline] = useState("for your time");
  const [closingContact, setClosingContact] = useState(defaultContact);
  const [note, setNote] = useState(DEFAULT_NOTE);
  const [items, setItems] = useState<DeckItem[]>([]);

  const submit = () =>
    start(async () => {
      setError(null);
      try {
        const slides = await buildSlides(items, note, { title, subtitle, closingHeading, closingTagline, closingContact, note });
        const res = await createPresentation({ title: [title, subtitle].filter(Boolean).join(" "), slides });
        if (res?.error) setError(res.error);
      } catch (e) {
        // A successful save redirects to the new deck; that is not a failure.
        if (e instanceof Error && e.message.includes("NEXT_REDIRECT")) return;
        setError(e instanceof Error ? e.message : "Could not create the presentation.");
      }
    });

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <div className="grid gap-4 lg:grid-cols-3">
        <section className="card space-y-3 p-4">
          <h2 className="text-sm font-semibold">Entry slide</h2>
          <div>
            <label className="label" htmlFor="title">
              Title
            </label>
            <input id="title" className="input mt-1" value={title} onChange={(e) => setTitle(e.target.value)} required />
          </div>
          <div>
            <label className="label" htmlFor="subtitle">
              Subtitle
            </label>
            <input id="subtitle" className="input mt-1" value={subtitle} onChange={(e) => setSubtitle(e.target.value)} />
          </div>
        </section>

        <section className="card space-y-3 p-4">
          <h2 className="text-sm font-semibold">Hamper & product slides</h2>
          <div>
            <label className="label" htmlFor="note">
              Terms note (bottom right)
            </label>
            <textarea id="note" rows={4} className="input mt-1" value={note} onChange={(e) => setNote(e.target.value)} />
          </div>
          <p className="text-xs text-[var(--color-muted)]">
            Each slide shows the product photos on the green band with their names, and the price with “+ Tax”.
          </p>
        </section>

        <section className="card space-y-3 p-4">
          <h2 className="text-sm font-semibold">Ending slide</h2>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="label" htmlFor="closingHeading">
                Heading
              </label>
              <input id="closingHeading" className="input mt-1" value={closingHeading} onChange={(e) => setClosingHeading(e.target.value)} />
            </div>
            <div>
              <label className="label" htmlFor="closingTagline">
                Tagline
              </label>
              <input id="closingTagline" className="input mt-1" value={closingTagline} onChange={(e) => setClosingTagline(e.target.value)} />
            </div>
          </div>
          <div>
            <label className="label" htmlFor="closingContact">
              Contact (under “For any Query”)
            </label>
            <textarea
              id="closingContact"
              rows={3}
              className="input mt-1"
              value={closingContact}
              onChange={(e) => setClosingContact(e.target.value)}
            />
          </div>
        </section>
      </div>

      <ItemPicker hampers={hampers} products={products} value={items} onChange={setItems} />

      {error && <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">{error}</p>}

      <div className="flex flex-wrap items-center gap-2">
        <button type="submit" className="btn-primary" disabled={pending || !items.length || !title.trim()}>
          {pending ? "Laying out slides…" : `Create presentation (${items.length + 2} slides)`}
        </button>
        <Link href="/presentations" className="btn-secondary">
          Cancel
        </Link>
      </div>
    </form>
  );
}
