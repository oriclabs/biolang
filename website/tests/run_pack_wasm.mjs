/**
 * Execute every example in a pack through the browser WASM module.
 *
 * A name-match against `builtins.json` only proves a function is registered.
 * This runs the real source through the real module, which is the only way to
 * know a pack is genuinely playable in the browser before shipping a "Run"
 * button that promises it is.
 *
 * Network examples are reported separately: the fetch bridge here is a stub,
 * so they are expected to fail and are not counted as regressions.
 *
 * Usage: node tests/run_pack_wasm.mjs [packId]
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(websiteRoot, "..");
const packId = process.argv[2] ?? "rosalind-armory";
const packDirectory = path.join(repositoryRoot, "packs", packId);
const examplesDirectory = path.join(packDirectory, "examples");

// Problems declared `network = true` cannot run against a stubbed fetch bridge.
const manifest = fs.readFileSync(path.join(packDirectory, "pack.toml"), "utf8");
const networkFiles = new Set();
for (const block of manifest.split(/\[\[problem\]\]/).slice(1)) {
  if (!/^\s*network\s*=\s*true\s*$/m.test(block)) continue;
  const file = block.match(/^\s*file\s*=\s*"([^"]+)"/m);
  if (file) networkFiles.add(path.basename(file[1]));
}

globalThis.window = globalThis;
globalThis.__blFiles = {};
globalThis.__blFetch = { sync: () => "ERROR:404 no network in this harness" };

const bytes = fs.readFileSync(path.join(websiteRoot, "wasm", "bl_wasm_bg.wasm"));
const wasm = await import("../wasm/bl_wasm.js");
await wasm.default({ module_or_path: bytes });
wasm.init();

let passed = 0;
const failures = [];
const networkSkipped = [];

for (const file of fs.readdirSync(examplesDirectory).sort()) {
  if (!file.endsWith(".bl")) continue;
  const source = fs.readFileSync(path.join(examplesDirectory, file), "utf8");

  wasm.reset();
  const started = Date.now();
  let raw;
  try {
    raw = JSON.parse(wasm.evaluate(source));
  } catch (error) {
    raw = { error: String(error) };
  }
  const elapsed = Date.now() - started;
  const error = raw.error ?? null;

  if (networkFiles.has(file)) {
    networkSkipped.push(file);
    console.log(`  skip  ${file.padEnd(10)} (network)`);
    continue;
  }

  if (error) {
    failures.push({ file, error });
    console.log(`  FAIL  ${file.padEnd(10)} ${String(error).split("\n")[0]}`);
    continue;
  }

  // Running without an error is not the same as getting the right answer.
  // Every solved example prints a `Match:` line, so hold WASM to it: a browser
  // that silently computes something different is worse than one that errors.
  const output = String(raw.output ?? raw.result ?? "");
  const verdicts = [...output.matchAll(/Match:\s*(true|false)/g)].map((m) => m[1]);
  const wrong = verdicts.filter((verdict) => verdict === "false").length;
  if (wrong > 0) {
    failures.push({ file, error: `${wrong} Match line(s) reported false` });
    console.log(`  WRONG ${file.padEnd(10)} ${wrong} Match line(s) false`);
    continue;
  }

  passed += 1;
  const checked = verdicts.length > 0 ? `, ${verdicts.length} verified` : "";
  console.log(`  ok    ${file.padEnd(10)} (${elapsed} ms${checked})`);
}

console.log(
  `\n${passed} passed, ${failures.length} failed, ` +
    `${networkSkipped.length} skipped as network-dependent`,
);
if (failures.length > 0) process.exit(1);
