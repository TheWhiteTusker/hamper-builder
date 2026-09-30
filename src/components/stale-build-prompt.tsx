"use client";

import { useEffect } from "react";

/**
 * A page left open across a deploy still calls the previous build's server
 * actions, which the new server doesn't have ("Server Action … was not
 * found"). Offer the reload that fixes it. Asks rather than reloading, so
 * unsaved edits on the page aren't thrown away.
 */
export function StaleBuildPrompt() {
  useEffect(() => {
    const original = window.fetch;
    let asked = false;
    window.fetch = async (...args) => {
      const res = await original(...args);
      if (!asked && res.headers.get("x-nextjs-action-not-found") === "1") {
        asked = true;
        if (confirm("Lattice Lane has been updated. Reload the page to continue?\n\nUnsaved changes on this page will be lost.")) {
          location.reload();
        }
      }
      return res;
    };
    return () => {
      window.fetch = original;
    };
  }, []);
  return null;
}
