// BioLang Viewer Service Worker — offline support
const CACHE_NAME = "biolang-viewer-v8";

// Bypass entirely on localhost — caching in dev causes stale asset headaches.
const IS_DEV = self.location.hostname === "localhost"
            || self.location.hostname === "127.0.0.1"
            || self.location.hostname === "0.0.0.0";

const ASSETS = [
  "/viewer.html",
  "/browser.html",
  "/studio.html",
  "/js/viewer.js",
  "/js/browser.js",
  "/js/main.js",
  "/js/somer-client.js",
  "/assets/styles.css",
  "/assets/favicon.svg",
  "/wasm/bl_wasm.js",
  "/wasm/bl_wasm_bg.wasm"
];

self.addEventListener("install", function(e) {
  if (IS_DEV) { self.skipWaiting(); return; }
  e.waitUntil(
    caches.open(CACHE_NAME).then(function(cache) {
      // Add assets individually so one missing file doesn't abort the whole install.
      return Promise.allSettled(
        ASSETS.map(function(url) { return cache.add(url); })
      );
    }).then(function() { self.skipWaiting(); })
  );
});

self.addEventListener("activate", function(e) {
  e.waitUntil(
    caches.keys().then(function(keys) {
      return Promise.all(
        keys.filter(function(k) { return k !== CACHE_NAME; })
            .map(function(k) { return caches.delete(k); })
      );
    })
  );
  self.clients.claim();
});

self.addEventListener("fetch", function(e) {
  // Dev: pass every request straight through — no caching, no interference.
  if (IS_DEV) return;

  // Only intercept same-origin http(s) GETs that aren't API calls.
  if (!e.request.url.startsWith("http")) return;
  var url = new URL(e.request.url);
  if (e.request.method !== "GET"
      || url.origin !== self.location.origin
      || e.request.headers.has("authorization")
      || url.pathname.startsWith("/v1/")) {
    return;
  }

  if (e.request.mode === "navigate") {
    // Network-first for HTML pages; fall back to cache when offline.
    e.respondWith(
      fetch(e.request).catch(function() {
        return caches.match(e.request).then(function(r) {
          return r || new Response("Offline — page not cached.", {
            status: 503,
            headers: { "Content-Type": "text/plain" },
          });
        });
      })
    );
  } else {
    // Cache-first for assets; populate cache on first network hit.
    e.respondWith(
      caches.match(e.request).then(function(cached) {
        if (cached) return cached;
        return fetch(e.request).then(function(res) {
          if (res.status === 200) {
            var clone = res.clone();
            caches.open(CACHE_NAME).then(function(cache) {
              cache.put(e.request, clone);
            });
          }
          return res;
        }).catch(function() {
          // Network unavailable and asset not cached — return empty 503.
          return new Response("", { status: 503 });
        });
      })
    );
  }
});
