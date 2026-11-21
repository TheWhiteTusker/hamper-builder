import pkg from "../../package.json";
import { getCloudflareContext } from "@opennextjs/cloudflare";

// Real only in the desktop app: CI sets package.json's patch to the run number
// for the desktop build alone, so the website's copy never moves.
export const APP_VERSION = pkg.version;

type Bucket = { get(key: string): Promise<{ json(): Promise<{ version?: string }> } | null> };

/** Newest published desktop release (latest.json in R2); null off Cloudflare. */
export async function latestDesktopVersion(): Promise<string | null> {
  try {
    const { env } = await getCloudflareContext({ async: true });
    const file = await (env as unknown as { DOWNLOADS?: Bucket }).DOWNLOADS?.get("latest.json");
    return (await file?.json())?.version ?? null;
  } catch {
    return null;
  }
}
