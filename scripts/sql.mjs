/**
 * Run ad-hoc SQL against the project in .env, as the service role.
 *
 *   node scripts/sql.mjs "select count(*) from products"
 *   echo "select 1" | node scripts/sql.mjs
 *
 * Uses SUPABASE_ACCESS_TOKEN, so it bypasses RLS. Handy for verification and
 * debugging; not something the app itself ever calls.
 */

import { readFile } from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");

const raw = await readFile(path.join(ROOT, ".env"), "utf8").catch(() => "");
const env = { ...process.env };
for (const line of raw.split("\n")) {
  const m = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$/);
  if (m) env[m[1]] = m[2].trim().replace(/^["']|["']$/g, "");
}

const ref = (env.NEXT_PUBLIC_SUPABASE_URL ?? "").match(
  /https:\/\/([a-z0-9]+)\.supabase\.co/,
)?.[1];

if (!ref || !env.SUPABASE_ACCESS_TOKEN) {
  console.error("Need NEXT_PUBLIC_SUPABASE_URL and SUPABASE_ACCESS_TOKEN in .env");
  process.exit(1);
}

const sql =
  process.argv.slice(2).join(" ").trim() ||
  (await new Promise((resolve) => {
    let buf = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (d) => (buf += d));
    process.stdin.on("end", () => resolve(buf));
  }));

if (!sql) {
  console.error("No SQL given.");
  process.exit(1);
}

const res = await fetch(`https://api.supabase.com/v1/projects/${ref}/database/query`, {
  method: "POST",
  headers: {
    Authorization: `Bearer ${env.SUPABASE_ACCESS_TOKEN}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({ query: sql }),
});

const text = await res.text();

if (!res.ok) {
  console.error(`HTTP ${res.status}`);
  console.error(text.slice(0, 2000));
  process.exit(1);
}

try {
  console.log(JSON.stringify(JSON.parse(text), null, 2));
} catch {
  console.log(text);
}
