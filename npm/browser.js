/**
 * BioLang for bundlers and browsers.
 *
 * Same API as the Node entry, minus the filesystem: there is no disk to read,
 * so the fetch hook goes to the network. A host that wants files — a workbench
 * with an in-memory workspace, say — can pass its own `fetch` implementation
 * rather than being forced through HTTP.
 */
import { BioLangSession } from "./session.js";
import { version } from "./version.js";

function createBridge(options) {
  const custom = options.fetchSync;
  globalThis.window ??= globalThis;
  globalThis.__blFiles ??= {};
  const bridge = {
    sync(url) {
      if (custom) {
        try {
          return custom(url);
        } catch (error) {
          return "ERROR:" + String(error?.message ?? error);
        }
      }
      // The interpreter calls this mid-evaluation and cannot await, so this is
      // a synchronous XMLHttpRequest. It is deprecated on the main thread and
      // unavailable in workers; supply `fetchSync` to avoid it.
      try {
        if (typeof XMLHttpRequest === "undefined") {
          return "ERROR:no fetchSync provided and XMLHttpRequest is unavailable";
        }
        const request = new XMLHttpRequest();
        request.open("GET", url, false);
        request.send(null);
        return request.status >= 200 && request.status < 300
          ? request.responseText
          : "ERROR:" + request.status + " for " + url;
      } catch (error) {
        return "ERROR:" + String(error?.message ?? error);
      }
    },
  };
  return () => { globalThis.__blFetch = bridge; };
}

export class BioLang extends BioLangSession {
  /**
   * @param {{ fetchSync?: (url: string) => string }} [options]
   *   fetchSync  synchronous reader for file and URL access
   */
  static async create(options = {}) {
    const activateBridge = createBridge(options);
    activateBridge();
    // Keep the 9 MB module out of the application's initial bundle. Browser
    // notebooks load it only when their first BioLang session is requested.
    const wasm = await import("./pkg-web/bl_wasm.js");
    await wasm.default();
    wasm.init();
    return new BioLang(wasm, new wasm.WasmSession(), activateBridge);
  }
}

export async function run(source, options = {}) {
  const bl = await BioLang.create(options);
  try {
    return bl.run(source);
  } finally {
    bl.dispose();
  }
}

export { version };

// Keep the package-root surface aligned with Node: colliding names resolve to
// generated WASM builtins; structural helpers remain under `biolang/dsl`.
export { dna, protein, range, rna, set, slice } from "./generated-builtins.js";
export * from "./dsl.js";
export * from "./generated-builtins.js";
export * from "./objects.js";
export * from "./somer.js";
