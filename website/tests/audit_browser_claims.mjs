/**
 * Cross-check the docs table's "Runs in" column against reality.
 *
 * The column is computed by static analysis of each source against the WASM
 * builtin catalog. That is a claim, not a measurement — so this runs every
 * problem through the real module and reports any disagreement in either
 * direction: a problem claimed browser-runnable that fails, or one written off
 * as CLI-only that actually works.
 *
 * Usage: node tests/audit_browser_claims.mjs
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(websiteRoot, "..");

const available = new Set(
  JSON.parse(fs.readFileSync(path.join(websiteRoot, "wasm", "builtins.json"), "utf8")).builtins,
);

const KEYWORDS = new Set([
  "if", "for", "while", "fn", "let", "return", "else", "then", "and", "or",
  "not", "try", "catch", "in", "assert", "import", "true", "false",
  "dna", "rna", "protein",
]);

function missingInBrowser(source) {
  const stripped = source
    .split(/\r?\n/)
    .map((line) => line.replace(/#.*$/, ""))
    .join("\n")
    .replace(/"([^"\\]|\\.)*"/g, '""');
  const declared = new Set([...stripped.matchAll(/\bfn\s+([A-Za-z_]\w*)/g)].map((m) => m[1]));
  const called = new Set([...stripped.matchAll(/\b([a-z_][a-z0-9_]*)\s*\(/g)].map((m) => m[1]));
  return [...called].filter(
    (n) => !KEYWORDS.has(n) && !declared.has(n) && !available.has(n),
  );
}

globalThis.window = globalThis;
globalThis.__blFiles = {};
globalThis.__blFetch = { sync: () => "ERROR:404 no network in this audit" };

const bytes = fs.readFileSync(path.join(websiteRoot, "wasm", "bl_wasm_bg.wasm"));
const wasm = await import("../wasm/bl_wasm.js");
await wasm.default({ module_or_path: bytes });
wasm.init();

const packsDir = path.join(repositoryRoot, "packs");
const disagreements = [];
let checked = 0;

for (const packId of fs.readdirSync(packsDir)) {
  const manifest = fs.readFileSync(path.join(packsDir, packId, "pack.toml"), "utf8");
  const networkIds = new Set();
  for (const block of manifest.split(/\[\[problem\]\]/).slice(1)) {
    if (/^\s*network\s*=\s*true\s*$/m.test(block)) {
      const id = block.match(/^\s*id\s*=\s*"([^"]+)"/m);
      if (id) networkIds.add(id[1]);
    }
  }

  const examples = path.join(packsDir, packId, "examples");
  for (const file of fs.readdirSync(examples).sort()) {
    if (!file.endsWith(".bl")) continue;
    const id = path.basename(file, ".bl").toUpperCase();
    if (networkIds.has(id)) continue; // cannot be judged without a live service

    const source = fs.readFileSync(path.join(examples, file), "utf8");
    const claimedBrowser = missingInBrowser(source).length === 0;

    wasm.reset();
    let ran = true;
    let error = null;
    try {
      const result = JSON.parse(wasm.evaluate(source));
      if (result.error) {
        ran = false;
        error = String(result.error).split("\n")[0];
      }
    } catch (thrown) {
      ran = false;
      error = String(thrown).split("\n")[0];
    }

    checked += 1;
    if (claimedBrowser !== ran) {
      disagreements.push({ packId, id, claimedBrowser, ran, error });
    }
  }
}

console.log(`Audited ${checked} problems against the real WASM module.`);
if (disagreements.length === 0) {
  console.log("Every 'Runs in' claim matches what actually happens.");
} else {
  console.error(`\n${disagreements.length} disagreement(s):`);
  for (const d of disagreements) {
    console.error(
      `  ${d.packId}/${d.id}: table says ${d.claimedBrowser ? "browser + CLI" : "CLI only"}, ` +
        `but it ${d.ran ? "runs" : "fails"} in the browser` +
        (d.error ? ` — ${d.error}` : ""),
    );
  }
  process.exit(1);
}
