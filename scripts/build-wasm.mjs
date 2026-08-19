#!/usr/bin/env node
/**
 * Build the browser runtime owned by the workbench and record what it was
 * built from. The website publisher copies this canonical module at deploy time.
 *
 * There were three manual steps here and each one had been forgotten at least
 * once: run wasm-pack, copy the output to desktop/public/wasm, and remember
 * that the module needed rebuilding at all. The first two are what
 * check-generated.mjs polices; the third is what check-wasm-fresh.mjs polices.
 * Both gates exist because this was a sequence of commands in a comment.
 *
 * Now it is one command, and it writes the fingerprint that lets the check know
 * whether a later commit has invalidated the module.
 *
 * Usage: node scripts/build-wasm.mjs
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { fingerprint } from "./lib/wasm-fingerprint.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workbenchWasm = path.join(repositoryRoot, "desktop", "public", "wasm");

console.log("Building the browser runtime (release)...");
execFileSync(
  "wasm-pack",
  [
    "build",
    "crates/bl-wasm",
    "--target",
    "web",
    "--out-dir",
    "../../desktop/public/wasm",
    "--no-typescript",
    "--release",
  ],
  { cwd: repositoryRoot, stdio: "inherit" },
);

// The fingerprint lives beside the generated directory rather than inside it: wasm-pack
// writes its own .gitignore into the out-dir on every build - a bare `*` -
// so anything new in there is untrackable, and an exception added by hand is
// destroyed by the next build.
//
mkdirSync(workbenchWasm, { recursive: true });
console.log(`Built the module in ${path.relative(repositoryRoot, workbenchWasm)}.`);

const { hash, fileCount } = fingerprint(repositoryRoot);
writeFileSync(
  path.join(repositoryRoot, "desktop", "public", "wasm-build-fingerprint.json"),
  `${JSON.stringify(
    {
      comment:
        "Written by scripts/build-wasm.mjs. Hash of the sources the committed "
        + "module was built from; scripts/check-wasm-fresh.mjs recomputes it to "
        + "detect a module that has fallen behind. Do not edit by hand.",
      sources: hash,
      fileCount,
      builtAt: new Date().toISOString(),
    },
    null,
    2,
  )}\n`,
);
console.log(`Recorded a fingerprint of ${fileCount} source files.`);
console.log("\nCommit desktop/public/wasm and its fingerprint together.");
