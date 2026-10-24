"use client";

import { useEffect } from "react";

/** Registers /sw.js (static build files only). Production only: in dev it would serve stale chunks. */
export function RegisterServiceWorker() {
  useEffect(() => {
    if (process.env.NODE_ENV === "production" && "serviceWorker" in navigator) {
      navigator.serviceWorker.register("/sw.js").catch(() => {});
    }
  }, []);
  return null;
}
