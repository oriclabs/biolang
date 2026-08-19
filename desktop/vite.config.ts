import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const here = path.dirname(fileURLToPath(import.meta.url));
const packsDir = path.resolve(here, "..", "dist", "packs");

/**
 * Serve the built example packs at /packs during `vite dev`.
 *
 * Example-pack deep links — /workbench/?pack=<id>&problem=<ID> — fetch the
 * catalog from /packs/index.json at the origin root. On lang.bio that is
 * /packs/, which the publisher builds and the site serves. The dev server's root is
 * desktop/public/, which has no packs directory, so every deep link 404'd on
 * the catalog fetch and opened the workbench with nothing in it.
 *
 * Mapping the directory in rather than copying it keeps one source of truth,
 * and means dev exercises the same code path as production instead of a
 * special-cased base URL.
 */
function servePacksInDev(): Plugin {
  return {
    name: "biolang-serve-packs",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const url = req.url ?? "";
        if (!url.startsWith("/packs/")) return next();

        // Strip the query and refuse anything that climbs out of the directory.
        const rel = decodeURIComponent(url.slice("/packs/".length).split("?")[0]);
        const target = path.resolve(packsDir, rel);
        if (!target.startsWith(packsDir)) {
          res.statusCode = 403;
          res.end("Forbidden");
          return;
        }

        if (!fs.existsSync(packsDir)) {
          // Say what is missing and how to make it, rather than a bare 404 the
          // app reports as "pack catalog unavailable".
          res.statusCode = 503;
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify({
            error: "Example packs have not been built",
            expected: packsDir,
            fix: "node scripts/build-packs.mjs",
          }, null, 2));
          return;
        }

        if (!fs.existsSync(target) || !fs.statSync(target).isFile()) return next();

        res.setHeader("Content-Type",
          target.endsWith(".json") ? "application/json" : "application/octet-stream");
        res.setHeader("Cache-Control", "no-cache");
        fs.createReadStream(target).pipe(res);
      });
    },
  };
}

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
  plugins: [react(), pwaServiceWorker(), servePacksInDev()],
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
    rollupOptions: {
      output: {
        manualChunks(id) {
          const normalized = id.replaceAll("\\", "/");
          if (normalized.includes("/monaco-editor/") || normalized.includes("/@monaco-editor/")) return "monaco";
          if (normalized.includes("/node_modules/react/")
            || normalized.includes("/node_modules/react-dom/")
            || normalized.includes("/node_modules/scheduler/")) return "react";
          if (normalized.includes("/node_modules/lucide-react/")) return "icons";
          if (normalized.includes("/src/generated/help-index.json")) return "help-index";
          if (normalized.includes("/src/generated/builtin-metadata.json")
            || normalized.includes("/src/generated/package-metadata.json")) return "biolang-metadata";
          return undefined;
        },
      },
    },
  },
});
