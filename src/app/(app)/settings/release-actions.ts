"use server";

import { getCloudflareContext } from "@opennextjs/cloudflare";
import { requireRole } from "@/lib/supabase/server";
import type { ActionState } from "@/lib/forms";

/**
 * Starts .github/workflows/desktop.yml, which builds the Windows app from main
 * and publishes it; installed apps then offer "Update now" on their next launch.
 * Needs the Worker secret GITHUB_TOKEN: a fine-grained token for this repo
 * with Actions: Read and write.
 */
const REPO = "https://api.github.com/repos/TheWhiteTusker/hamper-builder";
const WORKFLOW = `${REPO}/actions/workflows/desktop.yml`;

async function githubToken() {
  let token = process.env.GITHUB_TOKEN;
  try {
    const { env } = await getCloudflareContext({ async: true });
    token ??= (env as unknown as { GITHUB_TOKEN?: string }).GITHUB_TOKEN;
  } catch {
    // Not on Cloudflare (desktop app, local dev): process.env only.
  }
  return token;
}

const githubHeaders = (token: string) => ({
  authorization: `Bearer ${token}`,
  accept: "application/vnd.github+json",
  "user-agent": "lattice-lane",
});

export async function publishDesktopApp(): Promise<ActionState> {
  await requireRole("admin");
  const token = await githubToken();
  if (!token) return { error: "GITHUB_TOKEN is not set on the website." };

  // Skip the build when main hasn't moved since the last successful release.
  const headers = githubHeaders(token);
  const [head, last] = await Promise.all([
    fetch(`${REPO}/commits/main`, { headers, cache: "no-store" }),
    fetch(`${WORKFLOW}/runs?status=success&per_page=1`, { headers, cache: "no-store" }),
  ]);
  if (head.ok && last.ok) {
    const { sha } = (await head.json()) as { sha: string };
    const { workflow_runs } = (await last.json()) as { workflow_runs: { head_sha: string }[] };
    if (workflow_runs[0]?.head_sha === sha) {
      return { error: "Nothing new to package: the app already has the latest changes." };
    }
  }

  const res = await fetch(`${WORKFLOW}/dispatches`, {
    method: "POST",
    headers: githubHeaders(token),
    body: JSON.stringify({ ref: "main" }),
  });
  if (!res.ok) return { error: `GitHub refused (${res.status}): ${await res.text()}` };

  return { ok: true };
}

export type ReleaseStatus = {
  state: "running" | "success" | "failure" | "idle";
  percent: number;
  step?: string;
  url?: string;
  createdAt?: string;
};

type Run = {
  id: number;
  status: string;
  conclusion: string | null;
  created_at: string;
  run_started_at: string;
  updated_at: string;
  html_url: string;
};

/**
 * Progress of the latest desktop release. GitHub gives no percentage, so it is
 * estimated from elapsed time against the last successful run's duration, and
 * held at 99% until GitHub reports the run finished.
 */
export async function desktopReleaseStatus(): Promise<ReleaseStatus> {
  await requireRole("admin");
  const token = await githubToken();
  if (!token) return { state: "idle", percent: 0 };
  const headers = githubHeaders(token);

  const res = await fetch(`${WORKFLOW}/runs?per_page=10`, { headers, cache: "no-store" });
  if (!res.ok) return { state: "idle", percent: 0 };
  const runs = ((await res.json()) as { workflow_runs: Run[] }).workflow_runs;
  const run = runs[0];
  if (!run) return { state: "idle", percent: 0 };

  if (run.status === "completed") {
    return {
      state: run.conclusion === "success" ? "success" : "failure",
      percent: run.conclusion === "success" ? 100 : 0,
      url: run.html_url,
      createdAt: run.created_at,
    };
  }

  const lastGood = runs.find((r) => r.status === "completed" && r.conclusion === "success");
  const expectedMs = lastGood
    ? Date.parse(lastGood.updated_at) - Date.parse(lastGood.run_started_at)
    : 10 * 60_000;
  const elapsedMs = Date.now() - Date.parse(run.run_started_at);
  const percent = Math.min(99, Math.max(1, Math.round((elapsedMs / expectedMs) * 100)));

  let step: string | undefined;
  const jobs = await fetch(`${REPO}/actions/runs/${run.id}/jobs`, {
    headers,
    cache: "no-store",
  });
  if (jobs.ok) {
    const { jobs: list } = (await jobs.json()) as { jobs: { steps?: { name: string; status: string }[] }[] };
    step = list[0]?.steps?.find((s) => s.status === "in_progress")?.name;
  }

  return {
    state: "running",
    percent: run.status === "queued" ? 0 : percent,
    step,
    url: run.html_url,
    createdAt: run.created_at,
  };
}
