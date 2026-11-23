"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import {
  ACTIONS, DEFAULT_BINDINGS, assign, comboFromKey, comboFromMouse, comboLabel, loadBindings, saveBindings,
  subscribeBindings, type ActionId,
} from "@/lib/keybindings";
import { STUDIO_SHORTCUTS } from "@/components/studio/shortcut-list";

type Slot = { id: ActionId; slot: 0 | 1 };
const GROUPS = [...new Set(ACTIONS.map((a) => a.group))];

/**
 * Settings → Key bindings, like a game's controls screen: click a slot, then
 * press a key, a combination or a mouse button (middle, back, forward). Esc
 * cancels; Backspace clears. Saved on this device (lib/keybindings.ts).
 */
export function KeyBindings() {
  const bindings = useSyncExternalStore(subscribeBindings, loadBindings, () => DEFAULT_BINDINGS);
  const [waiting, setWaiting] = useState<Slot | null>(null);
  const [note, setNote] = useState("");

  useEffect(() => {
    if (!waiting) return;
    const put = (combo: string) => {
      const { bindings: next, movedFrom } = assign(loadBindings(), waiting.id, waiting.slot, combo);
      saveBindings(next);
      const from = ACTIONS.find((a) => a.id === movedFrom);
      setNote(from ? `${comboLabel(combo)} was on “${from.label}”; it's moved here.` : "");
      setWaiting(null);
    };
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const plain = !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey;
      if (e.key === "Escape") setWaiting(null);
      else if (plain && (e.key === "Backspace" || e.key === "Delete")) put("");
      else {
        const combo = comboFromKey(e);
        if (combo) put(combo);
      }
    };
    const onMouse = (e: MouseEvent) => {
      const combo = comboFromMouse(e.button);
      if (!combo) {
        if (e.type === "mousedown") setWaiting(null); // a normal click elsewhere cancels
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      if (e.type === "mouseup") put(combo);
    };
    const events = ["mousedown", "mouseup", "auxclick"] as const;
    window.addEventListener("keydown", onKey, true);
    events.forEach((t) => window.addEventListener(t, onMouse, true));
    return () => {
      window.removeEventListener("keydown", onKey, true);
      events.forEach((t) => window.removeEventListener(t, onMouse, true));
    };
  }, [waiting]);

  const isWaiting = (id: ActionId, slot: 0 | 1) => waiting?.id === id && waiting.slot === slot;

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_20rem]">
      <section className="card overflow-hidden">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3">
          <p className="text-xs text-(--color-muted)">
            Click a slot, then press a key, a combination or a mouse button. <kbd>Esc</kbd> cancels,{" "}
            <kbd>Backspace</kbd> clears. They work anywhere except while typing in a box, and are kept on this
            device: the website and the desktop app each have their own.
          </p>
          <button type="button" className="btn-secondary" onClick={() => (saveBindings(null), setNote("Back to the defaults."))}>
            Reset all
          </button>
        </div>
        {note && (
          <p role="status" className="border-b border-line bg-paper px-4 py-2 text-xs">
            {note}
          </p>
        )}
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-[11px] uppercase tracking-wide text-(--color-muted)">
              <th className="px-4 py-2 font-medium">Action</th>
              <th className="px-2 py-2 font-medium">Key</th>
              <th className="px-2 py-2 font-medium">Alternate</th>
            </tr>
          </thead>
          {GROUPS.map((group) => (
            <tbody key={group}>
              <tr>
                <th colSpan={3} className="bg-paper px-4 py-1.5 text-left text-xs font-semibold">
                  {group}
                </th>
              </tr>
              {ACTIONS.filter((a) => a.group === group).map((a) => (
                <tr key={a.id} className="border-t border-line">
                  <td className="px-4 py-1.5">{a.label}</td>
                  {([0, 1] as const).map((slot) => (
                    <td key={slot} className="px-2 py-1.5">
                      <button
                        type="button"
                        data-capturing={isWaiting(a.id, slot) || undefined}
                        onClick={() => setWaiting(isWaiting(a.id, slot) ? null : { id: a.id, slot })}
                        aria-label={`${a.label}, ${slot ? "alternate" : "key"}: ${bindings[a.id][slot] ? comboLabel(bindings[a.id][slot]) : "none"}. Click to change.`}
                        className={`min-w-36 rounded-md border px-2.5 py-1 text-left font-mono text-xs ${
                          isWaiting(a.id, slot)
                            ? "animate-pulse border-(--color-brand) bg-(--color-brand)/10 text-(--color-brand-dark)"
                            : "border-line bg-white hover:border-(--color-brand)"
                        }`}
                      >
                        {isWaiting(a.id, slot)
                          ? "Press a key or button…"
                          : bindings[a.id][slot]
                            ? comboLabel(bindings[a.id][slot])
                            : "—"}
                      </button>
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          ))}
        </table>
      </section>

      <section className="card self-start p-4">
        <h2 className="text-sm font-semibold">Hamper studio</h2>
        <p className="mt-0.5 mb-3 text-xs text-(--color-muted)">Fixed inside the canvas editor.</p>
        <dl className="space-y-1.5">
          {STUDIO_SHORTCUTS.map(([what, keys]) => (
            <div key={what} className="flex justify-between gap-3 text-xs">
              <dt className="text-(--color-muted)">{what}</dt>
              <dd className="text-right font-mono">{keys}</dd>
            </div>
          ))}
        </dl>
      </section>
    </div>
  );
}
