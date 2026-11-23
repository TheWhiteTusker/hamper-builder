"use client";

import { useEffect, useRef } from "react";
import { usePathname, useRouter } from "next/navigation";
import { record, redo, undo } from "@/lib/undo";

const NOT_TEXT = ["checkbox", "radio", "button", "submit", "reset", "range", "color", "file", "image"];

/** Focus is in a box with its own text undo (a just-clicked checkbox doesn't count). */
export const typingIn = (t: EventTarget | null) =>
  t instanceof HTMLElement &&
  (t.isContentEditable ||
    t.tagName === "TEXTAREA" ||
    (t instanceof HTMLInputElement && !NOT_TEXT.includes(t.type)));

/** Ctrl/⌘+Z is undo; Ctrl/⌘+Y and Ctrl/⌘+Shift+Z are redo. */
export function undoKey(e: KeyboardEvent): "undo" | "redo" | null {
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return null;
  const key = e.key.toLowerCase();
  if (key === "z") return e.shiftKey ? "redo" : "undo";
  if (key === "y" && !e.shiftKey) return "redo";
  return null;
}

/**
 * Undo and redo across the app, on the website and in the desktop app (same
 * pages). Steps (lib/undo.ts): ticks and review marks (selection.ts), and
 * each move to another screen, undone by going back and redone by going
 * forward. With nothing to undo, Ctrl+Z still goes back a screen.
 *
 * Text boxes keep their own undo. Screens with their own shortcut run first
 * and win by calling preventDefault: the studio canvas's undo/redo, and the
 * calculator's unsaved-changes prompt (leave-guard.tsx).
 *
 * ponytail: the browser's own Back/Forward buttons aren't steps, so after
 * using them a screen step can go back from a different page than it was
 * recorded on. Track history positions in history.state if that bites.
 */
export function UndoRedo() {
  const router = useRouter();
  const pathname = usePathname();
  const shown = useRef(pathname);
  const popped = useRef(false);

  useEffect(() => {
    const onPop = () => (popped.current = true);
    const onDown = (e: KeyboardEvent) => {
      const kind = undoKey(e);
      if (!kind || typingIn(e.target) || document.querySelector("dialog[open]")) return;
      // After every other keydown listener, whatever order they were added in.
      setTimeout(() => {
        if (e.defaultPrevented) return;
        if (kind === "redo") redo();
        else if (!undo() && history.length > 1) router.back();
      });
    };
    window.addEventListener("popstate", onPop);
    window.addEventListener("keydown", onDown);
    return () => {
      window.removeEventListener("popstate", onPop);
      window.removeEventListener("keydown", onDown);
    };
  }, [router]);

  // A move to another screen is a step, unless it was Back/Forward (ours or the browser's).
  useEffect(() => {
    if (shown.current === pathname) return;
    shown.current = pathname;
    if (popped.current) {
      popped.current = false;
      return;
    }
    record({ undo: () => router.back(), redo: () => router.forward() });
  }, [pathname, router]);

  return null;
}
