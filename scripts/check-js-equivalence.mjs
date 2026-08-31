#!/usr/bin/env node
/** Differential checks for the readable BioLang-to-JavaScript path. */
import assert from "node:assert/strict";
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
    const executable = generated.replace(
      /\nresult;\s*(?:\/\/[^\n]*)?\s*$/,
      "\nreturn result;",
    );
    const actual = await new Function(
      "bio",
      "bl",
      `return (async () => { ${executable} })()`,
    )(bio, bl);
    assert.deepEqual(actual, expected, `BioLang and JavaScript differ for: ${source}`);
  }
} finally {
  bl.dispose();
}

console.log(`${cases.length} readable JavaScript equivalence cases matched BioLang.`);
