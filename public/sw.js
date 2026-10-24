// Keeps the build's hashed files (/_next/static/*) on the device, so repeat
// visits and offline reloads don't re-download them. Names change whenever a
// file does, so a cached copy is never out of date.
//
// Deliberately NOT cached: pages and their data (per-user, behind login, and
// always network so a new release shows on the next visit), server actions and
// API routes, /updates/* downloads, and anything else on this domain. A fetch
// handler that doesn't call respondWith() leaves the request to the browser.
const CACHE = "static-v1";
const MAX_ENTRIES = 400;

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (e) => {
  const { request } = e;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== location.origin) return;
  if (!url.pathname.startsWith("/_next/static/")) return;
  e.respondWith(cacheFirst(request));
});

async function cacheFirst(request) {
  const cache = await caches.open(CACHE);
  const hit = await cache.match(request);
  if (hit) return hit;
  const res = await fetch(request);
  if (res.ok) {
    await cache.put(request, res.clone());
    // Old builds' files pile up; keys() is oldest-first, so trim from the front.
    const keys = await cache.keys();
    await Promise.all(keys.slice(0, Math.max(0, keys.length - MAX_ENTRIES)).map((k) => cache.delete(k)));
  }
  return res;
}
