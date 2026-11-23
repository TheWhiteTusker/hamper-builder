"use client";

import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";
import { usePathname, useRouter } from "next/navigation";
import { record, redo, undo } from "@/lib/undo";
import {
  ACTIONS, DEFAULT_BINDINGS, actionFor, comboFromKey, comboFromMouse, loadBindings, subscribeBindings, type ActionId,
} from "@/lib/keybindings";
import { mayLeave, type LeaveTo } from "./unsaved-warning";

const NOT_TEXT = ["checkbox", "radio", "button", "submit", "reset", "range", "color", "file", "image"];

/** Focus is in a box that takes typing (a just-clicked checkbox doesn't count). */
const typingIn = (t: EventTarget | null) =>
  t instanceof HTMLElement &&
  (t.isContentEditable ||
    t.tagName === "TEXTAREA" ||
    t.tagName === "SELECT" ||
    (t instanceof HTMLInputElement && !NOT_TEXT.includes(t.type)));

/** Settings → Key bindings is waiting for a key or button: leave it alone. */
const capturing = () => !!document.querySelector("[data-capturing]");

/**
 * The app's shortcuts, on the website and in the desktop app (same pages),
 * as bound in Settings → Key bindings (lib/keybindings.ts).
 *
 * Undo/redo steps (lib/undo.ts): ticks and review marks (selection.ts), and
 * each move to another screen, undone by going back and redone by going
 * forward. With nothing to undo, undo still goes back a screen.
 *
 * Typing in a box keeps every key for the box. Screens with their own use of
 * a key run first and win with preventDefault (the studio canvas's undo).
 * Before changing screen, screens with unsaved work may object (mayLeave).
 *
 * ponytail: the browser's own Back/Forward buttons aren't undo steps, so after
 * using them a screen step can go back from a different page than it was
 * recorded on. Track history positions in history.state if that bites.
 */
export function Shortcuts() {
  const router = useRouter();
  const pathname = usePathname();
  const shown = useRef(pathname);
  const popped = useRef(false);
  const bindings = useSyncExternalStore(subscribeBindings, loadBindings, () => DEFAULT_BINDINGS);

  const go = useCallback(
    (to: LeaveTo) => {
      if (!mayLeave(to)) return;
      if (to === "forward") router.forward();
      else if (to !== null) router.push(to);
      else if (history.length > 1) router.back();
    },
    [router],
  );

  useEffect(() => {
    const run = (id: ActionId) => {
      if (id === "undo") {
        if (!undo()) go(null);
      } else if (id === "redo") redo();
      else if (id === "back") go(null);
      else if (id === "forward") go("forward");
      else if (id === "search") {
        document.querySelector<HTMLInputElement>("main input[type=search], main input[name=q]")?.focus();
      } else {
        const href = ACTIONS.find((a) => a.id === id)?.href;
        if (href) go(href);
      }
    };

    const onKey = (e: KeyboardEvent) => {
      const id = actionFor(bindings, comboFromKey(e));
      if (!id || e.repeat || capturing() || typingIn(e.target) || document.querySelector("dialog[open]")) return;
      if (id === "search") e.preventDefault(); // not the browser's own find
      // After every other keydown listener, whatever order they were added in.
      setTimeout(() => !e.defaultPrevented && run(id));
    };

    // Middle and side buttons. Their browser defaults (back/forward, autoscroll,
    // open in a new tab) are stopped only when the button is bound.
    const onMouse = (e: MouseEvent) => {
      const id = actionFor(bindings, comboFromMouse(e.button));
      if (!id || capturing()) return;
      e.preventDefault();
      if (e.type === "mouseup") run(id);
    };

    const onPop = () => (popped.current = true);
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onMouse);
    window.addEventListener("mouseup", onMouse);
    window.addEventListener("auxclick", onMouse);
    window.addEventListener("popstate", onPop);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onMouse);
      window.removeEventListener("mouseup", onMouse);
      window.removeEventListener("auxclick", onMouse);
      window.removeEventListener("popstate", onPop);
    };
  }, [bindings, go]);

  // A move to another screen is an undo step, unless it was Back/Forward (ours or the browser's).
  useEffect(() => {
    if (shown.current === pathname) return;
    shown.current = pathname;
    if (popped.current) {
      popped.current = false;
      return;
    }
    record({ undo: () => go(null), redo: () => go("forward") });
  }, [pathname, go]);

  return null;
}
