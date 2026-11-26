import pkg from "../../package.json";

// Real only in the desktop app: CI sets package.json's patch to the run number
// for the desktop build alone, so the website's copy never moves.
export const APP_VERSION = pkg.version;

// The Rust (Tauri) app's updater manifest. "Update the app" now releases that
// app from main, so this is the release the button's work shows up in.
const LATEST = "https://github.com/TheWhiteTusker/hamper-builder/releases/latest/download/latest.json";

/** Newest published desktop release; null when GitHub can't be reached. */
export async function latestDesktopVersion(): Promise<string | null> {
  try {
    const res = await fetch(LATEST, { cache: "no-store" });
    return res.ok ? (((await res.json()) as { version?: string }).version ?? null) : null;
  } catch {
    return null;
  }
}
