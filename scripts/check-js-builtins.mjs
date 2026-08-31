#!/usr/bin/env node
/** Fail when the committed JavaScript builtin surface differs from WASM. */
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { WASM_API_COVERAGE } from "../npm/session.js";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasmRoot = path.join(root, "desktop", "public", "wasm");
const generated = JSON.parse(readFileSync(path.join(root, "npm", "wasm-builtins.json"), "utf8"));

globalThis.window = globalThis;
globalThis.__blFiles = {};
globalThis.__blFetch = { sync: () => "ERROR:offline JavaScript SDK check" };
const module_ = await import(pathToFileURL(path.join(wasmRoot, "bl_wasm.js")).href);
const wasmExports = Object.keys(module_).sort();
const coveredExports = Object.keys(WASM_API_COVERAGE).sort();
const uncoveredExports = wasmExports.filter((name) => !coveredExports.includes(name));
const staleExports = coveredExports.filter((name) => !wasmExports.includes(name));
if (uncoveredExports.length || staleExports.length) {
  console.error("The JavaScript SDK does not cover the current public WASM API.");
  if (uncoveredExports.length) console.error(`Uncovered exports: ${uncoveredExports.join(", ")}`);
  if (staleExports.length) console.error(`Stale coverage entries: ${staleExports.join(", ")}`);
  process.exit(1);
}
await module_.default({ module_or_path: readFileSync(path.join(wasmRoot, "bl_wasm_bg.wasm")) });
module_.init();

const actual = JSON.parse(module_.list_builtins()).map(({ name }) => name).sort();
const expected = generated.builtins.map(({ name }) => name).sort();
const missing = actual.filter((name) => !expected.includes(name));
const extra = expected.filter((name) => !actual.includes(name));
if (missing.length || extra.length) {
  console.error("The JavaScript SDK does not cover the current WASM builtin surface.");
  if (missing.length) console.error(`Missing: ${missing.join(", ")}`);
  if (extra.length) console.error(`Extra: ${extra.join(", ")}`);
  process.exit(1);
}
console.log(
  `JavaScript coverage is complete for all ${actual.length} WASM builtins `
  + `and all ${wasmExports.length} public WASM exports.`,
);
