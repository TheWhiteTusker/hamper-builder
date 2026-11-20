/**
 * Where a save or delete lands: the list as it was left (a submitted
 * `return_to`, filters kept), but only ever that list, never another site.
 * The "moved to Bin" banner is set for a delete and dropped otherwise.
 */
export function listReturn(kind: "products" | "hampers", value: unknown, deleted = false) {
  const url = new URL(String(value ?? ""), "http://x");
  const params = url.origin === "http://x" && url.pathname === `/${kind}` ? url.searchParams : new URLSearchParams();
  if (deleted) params.set("deleted", "1");
  else params.delete("deleted");
  const qs = params.toString();
  return `/${kind}${qs ? `?${qs}` : ""}`;
}
