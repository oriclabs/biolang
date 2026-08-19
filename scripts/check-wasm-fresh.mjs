#!/usr/bin/env node
/**
 * Fail if the committed WASM module does not expose what the runtime defines.
 *
 * `desktop/public/wasm/bl_wasm_bg.wasm` is a build artifact that lives in the
 * repository, and nothing rebuilds it automatically. Add a builtin to
 * crates/bl-runtime and every gate stays green: cargo test passes, clippy
 * passes, check-generated passes - and the playground and the workbench still
 * run the older runtime, silently, because a stale module loads and works
 * exactly like a current one.
 *
 * So this compares the shipped module against the source instead:
 *
 *   source   cargo run --example builtin-names --no-default-features
 *            (`native` is the only feature separating the browser build from
 *            the CLI, so no-default-features is the browser's registry)
 *   shipped  the module's own list_builtins(), read by running it in node -
 *            directly through its generated JavaScript loader
 *
 * A name in the source but not the module means the module predates it. A name
 * in the module but not the source means the module outlived a removal.
 *
 * Names are not the whole story, though, and the second check is there because
 * of what the first one missed. Rasterising large scatter plots added no
 * builtins and changed what an existing one returns; the name comparison called
 * the module current while the playground was still emitting a 65 MB SVG for a
 * plot the CLI drew in 37 KB. So the build also records a hash of every source
 * it compiled, and this recomputes it - catching any change, named or not.
 *
 * Either way the fix is one command, which rebuilds the workbench-owned module
 * and re-records the fingerprint:
 *
 *   node scripts/build-wasm.mjs
 *
 * Usage: node scripts/check-wasm-fresh.mjs
 */

import { execFile } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { promisify } from "node:util";

import { fingerprint } from "./lib/wasm-fingerprint.mjs";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const moduleRoot = path.join(repositoryRoot, "desktop", "public", "wasm");
const module_ = path.join(moduleRoot, "bl_wasm_bg.wasm");

const REBUILD = "node scripts/build-wasm.mjs";

if (!existsSync(module_)) {
  console.error(`Cannot check the browser runtime: ${module_} is missing.`);
  console.error(`Build it with:\n  ${REBUILD}`);
  process.exit(1);
}

/** What the Rust source says the browser build registers. */
async function builtinsInSource() {
  const { stdout } = await execFileAsync(
    "cargo",
    [
      "run",
      "--quiet",
      "--example",
      "builtin-names",
      "--package",
      "bl-runtime",
      "--no-default-features",
    ],
    { cwd: repositoryRoot, maxBuffer: 8 * 1024 * 1024 },
  );
  return new Set(stdout.split("\n").map((line) => line.trim()).filter(Boolean));
}

/**
 * What the committed module actually exposes.
 *
 * Load the committed module itself rather than trusting a generated catalog.
 */
async function builtinsInModule() {
  globalThis.window = globalThis;
  globalThis.__blFiles = {};
  globalThis.__blFetch = { sync: () => "ERROR:404 offline catalog generation" };
  const loader = pathToFileURL(path.join(moduleRoot, "bl_wasm.js")).href;
  const wasm = await import(loader);
  await wasm.default({ module_or_path: readFileSync(module_) });
  wasm.init();
  return new Set(
    JSON.parse(wasm.list_builtins()).map((entry) => entry.name).filter(Boolean),
  );
}

/**
 * Has any source the module is compiled from changed since it was built?
 *
 * The builtin comparison below only sees names. Rasterising large scatter plots
 * added no names and changed what umap_plot returns; this check reported the
 * module current while the playground still emitted a 65 MB SVG, and stayed
 * green through a rebuild that had silently failed from the wrong directory. A
 * check with a blind spot that wide is the sort of green tick that has cost this
 * repository a release already.
 *
 * Missing fingerprint means a module built before this existed, which is
 * old by definition.
 */
function fingerprintDrift() {
  const recorded = path.join(repositoryRoot, "desktop", "public", "wasm-build-fingerprint.json");
  const current = fingerprint(repositoryRoot);
  if (!existsSync(recorded)) {
    return { why: "the module carries no record of what it was built from", current };
  }
  const { sources } = JSON.parse(readFileSync(recorded, "utf8"));
  if (sources !== current.hash) {
    return {
      why: `the runtime sources have changed since the module was built (${current.fileCount} files hashed)`,
      current,
    };
  }
  return null;
}

const [source, shipped] = await Promise.all([builtinsInSource(), builtinsInModule()]);

const missing = [...source].filter((name) => !shipped.has(name)).sort();
const extra = [...shipped].filter((name) => !source.has(name)).sort();
const drift = fingerprintDrift();

if (missing.length || extra.length || drift) {
  console.error("\nThe committed WASM module is out of step with the runtime:\n");
  if (drift) {
    console.error(`  ${drift.why}.`);
    // Named separately because it is the case with no visible symptom: every
    // builtin is present and every one of them may behave differently.
    if (!missing.length && !extra.length) {
      console.error("    The builtin list still matches, so this is a behaviour change:");
      console.error("    the same functions, compiled from different code.\n");
    } else {
      console.error("");
    }
  }
  if (missing.length) {
    console.error(`  ${missing.length} builtin(s) defined in the source but absent from the module:`);
    console.error(`    ${missing.join(", ")}`);
    console.error("    The playground and the workbench cannot call these.\n");
  }
  if (extra.length) {
    console.error(`  ${extra.length} builtin(s) in the module that the source no longer defines:`);
    console.error(`    ${extra.join(", ")}`);
    console.error("    The module outlived a removal.\n");
  }
  console.error(`  fix: ${REBUILD}`);
  console.error("       (rebuilds the workbench module and re-records the fingerprint)\n");
  process.exit(1);
}

console.log(
  `The browser runtime is current: ${shipped.size} builtins, matching the source exactly.`,
);
