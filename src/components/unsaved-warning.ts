"use client";

import { useEffect } from "react";

/**
 * Sent before a shortcut changes screen (shortcuts.tsx). A screen with unsaved
 * work cancels it with preventDefault. `to`: a path, null for back, "forward".
 */
export const LEAVE_EVENT = "lattice:leave";
export type LeaveTo = string | null | "forward";

/** True when no screen objects to leaving. */
export const mayLeave = (to: LeaveTo) =>
  window.dispatchEvent(new CustomEvent<LeaveTo>(LEAVE_EVENT, { cancelable: true, detail: to }));

/** While `dirty`: warn before closing or reloading the tab, or a shortcut leaving the screen. */
export function useUnsavedWarning(dirty: boolean) {
  useEffect(() => {
    if (!dirty) return;
    const unload = (e: BeforeUnloadEvent) => e.preventDefault();
    const leave = (e: Event) => {
      if (!confirm("Leave without saving? Your changes will be lost.")) e.preventDefault();
    };
    window.addEventListener("beforeunload", unload);
    window.addEventListener(LEAVE_EVENT, leave);
    return () => {
      window.removeEventListener("beforeunload", unload);
      window.removeEventListener(LEAVE_EVENT, leave);
    };
  }, [dirty]);
}
