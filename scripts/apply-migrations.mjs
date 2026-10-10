/**
 * Applies supabase/migrations/*.sql to the project in .env, in order.
 *
 *   node scripts/apply-migrations.mjs          # apply
 *   node scripts/apply-migrations.mjs --check  # report what already exists
 *
 * Needs a Supabase personal access token, because the publishable key in
 * .env is RLS-scoped and cannot run DDL. Create one at
 * https://supabase.com/dashboard/account/tokens and add it to .env:
 *
 *   SUPABASE_ACCESS_TOKEN=sbp_...
 *
 * .env is gitignored, so the token stays on this machine.
 */

import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const CHECK_ONLY = process.argv.includes("--check");

async function loadEnv() {
  const raw = await readFile(path.join(ROOT, ".env"), "utf8").catch(() => "");
  const env = { ...process.env };

  for (const line of raw.split("\n")) {
    const m = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$/);
    if (m) env[m[1]] = m[2].trim().replace(/^["']|["']$/g, "");
  }
  return env;
}

async function query(ref, token, sql) {
  const res = await fetch(`https://api.supabase.com/v1/projects/${ref}/database/query`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: sql }),
  });

  const text = await res.text();
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${text.slice(0, 400)}`);

  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

const env = await loadEnv();
const token = env.SUPABASE_ACCESS_TOKEN;
const url = env.NEXT_PUBLIC_SUPABASE_URL ?? "";
const ref = url.match(/https:\/\/([a-z0-9]+)\.supabase\.co/)?.[1];

if (!ref) {
  console.error("Could not read a project ref from NEXT_PUBLIC_SUPABASE_URL in .env");
  process.exit(1);
}

if (!token) {
  console.error(
    "SUPABASE_ACCESS_TOKEN is not set in .env.\n" +
      "Create one at https://supabase.com/dashboard/account/tokens and add:\n" +
      "  SUPABASE_ACCESS_TOKEN=sbp_...",
  );
  process.exit(1);
}

console.log(`Project: ${ref}\n`);

// What is already there? Re-running a migration that has partly applied is the
// main way to make a mess, so report before touching anything.
const existing = await query(
  ref,
  token,
  `select table_name from information_schema.tables
    where table_schema = 'public' order by table_name`,
);

const tables = (Array.isArray(existing) ? existing : []).map((r) => r.table_name);
console.log(
  tables.length ? `Existing public tables/views: ${tables.join(", ")}` : "Schema is empty.",
);

if (CHECK_ONLY) process.exit(0);

if (tables.length) {
  console.error(
    "\nRefusing to run: the schema is not empty.\n" +
      "0001-0004 create objects and would fail on a re-run. Either drop them first,\n" +
      "or apply only the migration you actually need.",
  );
  process.exit(1);
}

const dir = path.join(ROOT, "supabase", "migrations");
const files = (await readdir(dir)).filter((f) => f.endsWith(".sql")).sort();

for (const file of files) {
  const sql = await readFile(path.join(dir, file), "utf8");
  process.stdout.write(`  ${file} … `);

  try {
    await query(ref, token, sql);
    console.log("ok");
  } catch (error) {
    console.log("FAILED");
    console.error(`\n${error.message}\n`);
    process.exit(1);
  }
}

const after = await query(
  ref,
  token,
  `select table_name from information_schema.tables
    where table_schema = 'public' order by table_name`,
);

console.log(
  `\nDone. Created: ${(Array.isArray(after) ? after : []).map((r) => r.table_name).join(", ")}`,
);
