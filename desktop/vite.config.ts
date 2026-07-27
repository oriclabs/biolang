import { createHash } from "node:crypto";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

function pwaServiceWorker(): Plugin {
  return {
    name: "biolang-pwa-service-worker",
    apply: "build",
    generateBundle(_options, bundle) {
      const files = [
        "./",
        "./index.html",
        "./manifest.webmanifest",
        "./icons/icon-192.png",
        "./icons/icon-512.png",
        "./wasm/bl_wasm.js",
        "./wasm/bl_wasm_bg.wasm",
        ...Object.keys(bundle).sort().map((name) => `./${name}`),
      ];
      const version = createHash("sha256").update(files.join("\n")).digest("hex").slice(0, 12);
      const source = `
const CACHE_NAME = "biolang-studio-${version}";
const PRECACHE = ${JSON.stringify(files, null, 2)};

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then((cache) => cache.addAll(PRECACHE.map((path) => new URL(path, self.registration.scope))))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((key) => key.startsWith("biolang-studio-") && key !== CACHE_NAME).map((key) => caches.delete(key))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (response.ok) {
            const copy = response.clone();
            void caches.open(CACHE_NAME).then((cache) => cache.put(new URL("./index.html", self.registration.scope), copy));
          }
          return response;
        })
        .catch(() => caches.match(new URL("./index.html", self.registration.scope)))
    );
    return;
  }

  const inAppBundle =
    url.pathname.includes("/assets/") ||
    url.pathname.includes("/icons/") ||
    url.pathname.includes("/wasm/") ||
    url.pathname.endsWith("/manifest.webmanifest");
  if (!inAppBundle) return;
  event.respondWith(
    caches.match(request).then((cached) => cached || fetch(request).then((response) => {
      if (response.ok) {
        const copy = response.clone();
        void caches.open(CACHE_NAME).then((cache) => cache.put(request, copy));
      }
      return response;
    }))
  );
});
`;
      this.emitFile({ type: "asset", fileName: "sw.js", source });
    },
  };
}

export default defineConfig({
  base: "./",
  plugins: [react(), pwaServiceWorker()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
  },
});
