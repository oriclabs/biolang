/**
 * BioLang for Node.
 *
 * Wraps the WebAssembly build in something ergonomic: one class, results as
 * objects rather than JSON strings, and the file bridge installed for you.
 *
 * The raw module expects a host-provided `__blFetch.sync` hook for anything
 * that reads a file or a URL — it was written for the browser, where those go
 * through the page. Under Node that hook has to come from somewhere, and every
 * caller writing their own is how a small thing becomes a support burden. This
 * installs a Node implementation: local paths read from disk, http(s) fetched
 * synchronously.
 */
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

/**
 * The hook is synchronous because the interpreter calls it mid-evaluation and
 * cannot await. Node has no synchronous HTTP, so remote reads shell out to
 * curl. Local files — the common case — never pay that cost.
 */
function installBridge(options) {
  const allowNetwork = options.network !== false;
  const cwd = options.cwd ?? process.cwd();

  globalThis.window ??= globalThis;
  globalThis.__blFiles ??= {};
  globalThis.__blFetch = {
    sync(url) {
      try {
        if (/^https?:\/\//.test(url)) {
          if (!allowNetwork) {
            return "ERROR:network access is disabled (pass { network: true })";
          }
          return execFileSync("curl", ["-fsSL", "--max-time", "30", url], {
            encoding: "utf8",
            maxBuffer: 256 * 1024 * 1024,
          });
        }
        return fs.readFileSync(path.resolve(cwd, url), "utf8");
      } catch (error) {
        return "ERROR:" + String(error?.message ?? error);
      }
    },
  };
}

/** A BioLang interpreter instance. */
export class BioLang {
  #wasm;

  constructor(wasm) {
    this.#wasm = wasm;
  }

  /**
   * Load the WebAssembly module and return an interpreter.
   *
   * @param {{ cwd?: string, network?: boolean }} [options]
   *   cwd      base directory for relative paths (default: process.cwd())
   *   network  allow http(s) reads (default: true)
   */
  static async create(options = {}) {
    installBridge(options);
    const wasm = await import("./pkg-node/bl_wasm.js");
    wasm.init();
    return new BioLang(wasm);
  }

  /**
   * Run BioLang source.
   *
   * State persists between calls on the same instance, so a variable defined in
   * one `run` is visible in the next. Call `reset()` for a clean interpreter.
   */
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

  /** Discard all variables and start from a fresh interpreter. */
  reset() {
    this.#wasm.reset();
  }

  /** Every builtin available in this build: {name, signature, category}. */
  builtins() {
    return JSON.parse(this.#wasm.list_builtins());
  }

  /** Variables currently defined. */
  variables() {
    return JSON.parse(this.#wasm.list_variables());
  }

  /** Rewrite source into the canonical layout. */
  format(source, indent = 4) {
    return this.#wasm.format(source, indent);
  }

  /** Tokenise source, for editors and highlighting. */
  tokenize(source) {
    return JSON.parse(this.#wasm.tokenize(source));
  }

  /** Convert Python, R, Jupyter or R Markdown source to BioLang. */
  import(source, format, filename = "input") {
    return JSON.parse(this.#wasm.import_source(source, format, filename));
  }

  /** The underlying wasm-bindgen module, for anything not wrapped here. */
  get raw() {
    return this.#wasm;
  }
}

/** Convenience: load and run once. */
export async function run(source, options = {}) {
  const bl = await BioLang.create(options);
  return bl.run(source);
}

export const version = JSON.parse(
  fs.readFileSync(path.join(here, "package.json"), "utf8"),
).version;
