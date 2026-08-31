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
import { BioLangSession } from "./session.js";

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
export class BioLang extends BioLangSession {

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
}

/** Convenience: load and run once. */
export async function run(source, options = {}) {
  const bl = await BioLang.create(options);
  return bl.run(source);
}

export const version = JSON.parse(
  fs.readFileSync(path.join(here, "package.json"), "utf8"),
).version;

export * from "./dsl.js";
export * from "./generated-builtins.js";
export * from "./objects.js";
export * from "./somer.js";
