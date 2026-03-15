// BioLang Viewer Service Worker — offline support
const CACHE_NAME = "biolang-viewer-v5";
const ASSETS = [
  "/viewer.html",
  "/browser.html",
  "/studio.html",
  "/js/viewer.js",
  "/js/browser.js",
  "/js/main.js",
  "/assets/styles.css",
  "/assets/favicon.svg",
  "/wasm/bl_wasm.js",
  "/wasm/bl_wasm_bg.wasm"
];

self.addEventListener("install", function(e) {
  e.waitUntil(
    caches.open(CACHE_NAME).then(function(cache) {
      return cache.addAll(ASSETS);
    })
  );
  self.skipWaiting();
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
  // Only handle http/https — ignore chrome-extension://, etc.
  if (!e.request.url.startsWith("http")) return;

  // Network-first for HTML, cache-first for assets
  if (e.request.mode === "navigate") {
    e.respondWith(
      fetch(e.request).catch(function() {
        return caches.match(e.request);
      })
    );
  } else {
    e.respondWith(
      caches.match(e.request).then(function(r) {
        return r || fetch(e.request).then(function(res) {
          // Cache successful fetches
          if (res.status === 200) {
            var clone = res.clone();
            caches.open(CACHE_NAME).then(function(cache) {
              cache.put(e.request, clone);
            });
          }
          return res;
        });
      })
    );
  }
});
