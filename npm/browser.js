/**
 * BioLang for bundlers and browsers.
 *
 * Same API as the Node entry, minus the filesystem: there is no disk to read,
 * so the fetch hook goes to the network. A host that wants files — a workbench
 * with an in-memory workspace, say — can pass its own `fetch` implementation
 * rather than being forced through HTTP.
 */
import * as wasm from "./pkg-bundler/bl_wasm.js";

function installBridge(options) {
  const custom = options.fetchSync;
  globalThis.window ??= globalThis;
  globalThis.__blFiles ??= {};
  globalThis.__blFetch = {
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
}

export class BioLang {
  #wasm;

  constructor(module) {
    this.#wasm = module;
  }

  /**
   * @param {{ fetchSync?: (url: string) => string }} [options]
   *   fetchSync  synchronous reader for file and URL access
   */
  static async create(options = {}) {
    installBridge(options);
    wasm.init();
    return new BioLang(wasm);
  }

  run(source) {
    const parsed = JSON.parse(this.#wasm.evaluate(source));
    return {
      ok: parsed.ok ?? false,
      value: parsed.value ?? null,
      type: parsed.type ?? null,
      output: parsed.output ?? "",
      structured: parsed.structured ?? null,
      results: parsed.results ?? [],
      trace: parsed.trace ?? [],
      error: parsed.error ?? null,
    };
  }

  reset() {
    this.#wasm.reset();
  }

  builtins() {
    return JSON.parse(this.#wasm.list_builtins());
  }

  variables() {
    return JSON.parse(this.#wasm.list_variables());
  }

  format(source, indent = 4) {
    return this.#wasm.format(source, indent);
  }

  tokenize(source) {
    return JSON.parse(this.#wasm.tokenize(source));
  }

  import(source, format, filename = "input") {
    return JSON.parse(this.#wasm.import_source(source, format, filename));
  }

  get raw() {
    return this.#wasm;
  }
}

export async function run(source, options = {}) {
  const bl = await BioLang.create(options);
  return bl.run(source);
}
