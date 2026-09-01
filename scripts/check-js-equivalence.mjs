#!/usr/bin/env node
/** Differential checks for the readable BioLang-to-JavaScript path. */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { isDeepStrictEqual } from "node:util";
import * as bio from "../npm/dsl.js";
import { BioLang } from "../npm/index.js";

const cases = [
  "mean([1, 2, 3])",
  'gc_content(dna("ATGC"))',
  'gc_content(dna"ATGC")',
  'iupac_match(dna"AAATTC", "RAATTC")',
  "round(mean([1, 2, 3, 8]), 2)",
  "[mean([1, 2, 3]), median([1, 5, 9])]",
  "let report = summary([1, 2, 3])\nreport.mean",
  "pipeline qc { stage raw -> [1, 2, 3] stage count -> len(raw) }\nqc",
  "pipeline qc(values) { stage count -> len(values) }\nqc([1, 2, 3])",
];

const bl = await BioLang.create({ network: false });
try {
  for (const source of cases) {
    const expected = bl.evalValue(source);
    const generated = bl.transpileJavaScript(source);
    assert.doesNotMatch(
      generated,
      /bio\.program\(/,
      `equivalence fixture unexpectedly used structural mode: ${source}`,
    );
    const actual = await evaluateGenerated(generated, bl);
    assert.deepEqual(actual, expected, `BioLang and JavaScript differ for: ${source}`);
  }

  const corpus = trackedBioLangFiles();
  const report = await compareCorpus(corpus, bl);
  assert.equal(
    report.mismatches.length,
    0,
    report.mismatches.map(item => `${item.path}: ${item.error}`).join("\n"),
  );
  assert.ok(report.compared >= 100, `only ${report.compared} corpus programs were comparable`);
  console.log(
    `${report.compared} deterministic corpus programs matched; ` +
    `${report.runtimeSkipped} needed unavailable I/O/runtime state, ` +
    `${report.nondeterministic} were non-deterministic, and ` +
    `${report.staticSkipped} were outside the bounded execution set.`,
  );
} finally {
  bl.dispose();
}

console.log(`${cases.length} readable JavaScript equivalence cases matched BioLang.`);

function trackedBioLangFiles() {
  return execFileSync("git", ["ls-files", "-z", "*.bl"], {
    cwd: new URL("..", import.meta.url),
  }).toString("utf8").split("\0").filter(Boolean);
}

function eligibleCorpusSource(source) {
  if (source.length > 10_000) return false;
  // Keep this CI pass bounded and side-effect free. Unsupported runtime state
  // is measured separately below; potentially unbounded loops and host I/O
  // are not started in the first place.
  return !/(?:^|\n)\s*(?:import|from)\s/m.test(source)
    && !/\bwhile\b/.test(source)
    && !/\b(?:read_|write_|fetch_|save_|download|blast|checkpoint|bench|time_it|timestamp|sleep|exit|to_stream|stream_|par_)\w*\s*\(/.test(source);
}

async function compareCorpus(paths, session) {
  const report = {
    compared: 0,
    staticSkipped: 0,
    runtimeSkipped: 0,
    nondeterministic: 0,
    mismatches: [],
  };
  for (const path of paths) {
    const source = readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
    if (!eligibleCorpusSource(source)) {
      report.staticSkipped += 1;
      continue;
    }
    const generated = session.transpileJavaScript(source);
    if (!generated.startsWith("// Direct JavaScript API;")) {
      report.staticSkipped += 1;
      continue;
    }
    if (process.env.BIOLANG_EQUIVALENCE_PROGRESS) console.error(`equivalence: ${path}`);
    const first = evaluateBioLang(source, session);
    if (!first.ok) {
      report.runtimeSkipped += 1;
      continue;
    }
    const second = evaluateBioLang(source, session);
    if (!second.ok || !isDeepStrictEqual(first.value, second.value)) {
      report.nondeterministic += 1;
      continue;
    }
    try {
      resetAndSeed(session);
      const actual = await evaluateGenerated(generated, session);
      if (!isDeepStrictEqual(actual, first.value)) {
        report.mismatches.push({
          path,
          error: `expected ${preview(first.value)}, got ${preview(actual)}`,
        });
      } else {
        report.compared += 1;
      }
    } catch (error) {
      report.mismatches.push({ path, error: error instanceof Error ? error.message : String(error) });
    }
  }
  return report;
}

function evaluateBioLang(source, session) {
  try {
    resetAndSeed(session);
    return { ok: true, value: session.evalValue(source) };
  } catch (error) {
    return { ok: false, error };
  }
}

function resetAndSeed(session) {
  session.reset();
  session.callValue("set_seed", [0x5eed]);
}

function evaluateGenerated(generated, session) {
  const executable = generated.replace(
    /\n([A-Za-z_$][A-Za-z0-9_$]*|null);\s*((?:\/\/[^\n]*\s*)*)$/,
    "\nreturn $1;\n$2",
  );
  return new Function(
    "bio",
    "bl",
    `return (async () => { ${executable}\n})()`,
  )(bio, session);
}

function preview(value) {
  try {
    const rendered = JSON.stringify(value);
    return (rendered ?? String(value)).slice(0, 240);
  } catch {
    return String(value).slice(0, 240);
  }
}
