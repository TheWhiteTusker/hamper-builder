"use client";

import { useActionState, useEffect, useState } from "react";
import { publishDesktopApp, desktopReleaseStatus, type ReleaseStatus } from "../release-actions";

const STEP_LABELS: Record<string, string> = {
  "Run pnpm install --frozen-lockfile": "Installing dependencies",
  "Run pnpm build:desktop": "Building the app",
  "Run pnpm publish:desktop": "Publishing",
};

type ElectronBridge = { electron: { checkForUpdates: () => Promise<unknown> } };

/**
 * Website: starts a desktop release and shows its progress. Desktop app: checks
 * for updates; the prompts, download progress and Install now / Later all come
 * from electron/updater.cjs. Publishing needs the Worker's GITHUB_TOKEN, which the
 * app's local server never has, hence the split.
 */
export function UpdateAppButton({ desktop }: { desktop: boolean }) {
  return desktop ? <CheckForUpdatesButton /> : <PublishReleaseButton />;
}

function CheckForUpdatesButton() {
  const [checking, setChecking] = useState(false);

  async function check() {
    setChecking(true);
    try {
      await (window as unknown as ElectronBridge).electron.checkForUpdates();
    } finally {
      setChecking(false);
    }
  }

  return (
    <button type="button" onClick={check} className="btn-secondary" disabled={checking}>
      {checking ? "Checking…" : "Check for updates"}
    </button>
  );
}

function PublishReleaseButton() {
  const [status, setStatus] = useState<ReleaseStatus | null>(null);
  // When this page started a release. Runs created before it are an earlier
  // release, not this click's result (30s allows for clock skew with GitHub).
  const [since, setSince] = useState<number | null>(null);
  const [state, action, pending] = useActionState(async () => {
    const result = await publishDesktopApp();
    if (result.ok) setSince(Date.now() - 30_000);
    return result;
  }, {});

  const isFresh = (s: ReleaseStatus | null) =>
    since !== null && !!s?.createdAt && Date.parse(s.createdAt) >= since;

  // Poll while a release is running, including one started elsewhere, and
  // while waiting for a just-dispatched run to appear.
  useEffect(() => {
    let stop = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function poll() {
      const s = await desktopReleaseStatus().catch(() => null);
      if (stop) return;
      setStatus(s);
      const waiting = since !== null && !(s?.createdAt && Date.parse(s.createdAt) >= since);
      if (s?.state === "running" || waiting) timer = setTimeout(poll, 5000);
    }
    poll();
    return () => {
      stop = true;
      clearTimeout(timer);
    };
  }, [since]);

  const running = status?.state === "running";
  const fresh = isFresh(status);
  const starting = since !== null && !fresh;

  return (
    <div className="flex flex-wrap items-center gap-2.5">
      <form action={action}>
        <button
          type="submit"
          className="btn-secondary"
          disabled={pending || running || starting}
          title="Trigger GitHub Actions to compile current web version into Windows installer"
        >
          {pending ? "Starting…" : "Package web into app release"}
        </button>
      </form>
      {starting && !running && <span className="text-xs text-(--color-muted)">Starting on GitHub…</span>}
      {running && (
        <div className="flex items-center gap-2" role="status" aria-live="polite">
          <div className="h-2 w-32 overflow-hidden rounded bg-slate-200">
            <div
              className="h-2 bg-(--color-brand) transition-[width] duration-1000"
              style={{ width: `${status.percent}%` }}
            />
          </div>
          <span className="text-xs text-(--color-muted)">
            Packaging {status.percent}% · {(status.step && STEP_LABELS[status.step]) ?? "Preparing"}
          </span>
        </div>
      )}
      {fresh && status?.state === "success" && (
        <span className="text-sm text-green-800">Released. Everyone&rsquo;s app will offer the update.</span>
      )}
      {fresh && status?.state === "failure" && (
        <a href={status.url} target="_blank" rel="noreferrer" className="text-sm text-red-700 underline">
          Packaging failed. See the log on GitHub
        </a>
      )}
      {state.error && (
        <span role="alert" className="text-sm text-red-700">
          {state.error}
        </span>
      )}
    </div>
  );
}
