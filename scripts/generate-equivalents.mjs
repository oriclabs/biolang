#!/usr/bin/env node
/**
 * Generate the verified-equivalents page.
 *
 * The page shows the same computation in BioLang, JavaScript, Python and R.
 * The JavaScript pane is not written by hand: it is produced by the shipped
 * transpiler from the BioLang column, and is emitted only when running it
 * through the SDK returns the same decoded value that BioLang returns. A case
 * whose JavaScript needs structural builders, or whose result does not match,
 * simply gets no JavaScript tab rather than an unverified one.
 * Every trio on it comes from benchmarks/correctness/oneliners/cases.tsv, which
 * the correctness suite runs in all three languages and compares — so a tabbed
 * block here is one that CI has shown produces identical values, not a
 * translation somebody wrote out and hoped about.
 *
 * That constraint is the point. Writing Python and R equivalents for the ~1300
 * code blocks on the site by hand would mean 2600 unverified snippets sitting
 * beside BioLang examples that are checked on every commit. Generating from the
 * verified set instead means coverage grows with the correctness suite, one
 * line of cases.tsv at a time.
 *
 *   node scripts/generate-equivalents.mjs
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { BioLang } from "../npm/index.js";
import { requireSiteRoot } from "./lib/site-root.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CASES = path.join(ROOT, "benchmarks", "correctness", "oneliners", "cases.tsv");
const RESULTS = path.join(ROOT, "benchmarks", "correctness", "results", "oneliners.json");
const OUT = path.join(requireSiteRoot(ROOT), "docs", "examples", "equivalents.html");
const EVIDENCE_URL =
  "https://github.com/oriclabs/biolang/blob/main/benchmarks/correctness/results/oneliners.md";

const NL = "\n";

const escape = (s) =>
  String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

function loadCases() {
  const lines = fs.readFileSync(CASES, "utf8").split(/\r?\n/);
  let header = null;
  const rows = [];
  for (const line of lines) {
    if (!line.trim() || line.trimStart().startsWith("#")) continue;
    const parts = line.split("\t");
    if (!header) { header = parts; continue; }
    const row = {};
    header.forEach((h, i) => { row[h] = parts[i] ?? ""; });
    if (row.name && row.biolang) rows.push(row);
  }
  return rows;
}

function loadResults() {
  if (!fs.existsSync(RESULTS)) return null;
  try {
    const parsed = JSON.parse(fs.readFileSync(RESULTS, "utf8"));
    const byName = new Map();
    for (const r of parsed.results ?? []) byName.set(r.name, r);
    return byName;
  } catch {
    return null;
  }
}


// A bare expression shows what to call but does not print anything, so the Run
// button would appear to do nothing, and a copied Python snippet without its
// import will not run at all. Wrap each expression in the language's print and
// prepend only the imports that expression actually needs, so every pane is
// something you can paste and run where it belongs.
function pythonSetup(expr) {
  const lines = [];
  if (/statistics\./.test(expr)) lines.push("import statistics");
  if (/math\./.test(expr)) lines.push("import math");
  if (/Seq\(/.test(expr)) lines.push("from Bio.Seq import Seq");
  if (/gc_fraction\(/.test(expr)) lines.push("from Bio.SeqUtils import gc_fraction");
  // Python has no Levenshtein in the standard library. The reference is a plain
  // DP table, and the pane has to carry it or the snippet is uncopyable — these
  // three cases previously used the expected number as the "reference", which
  // rendered as print(0) and compared BioLang against a typed-in constant
  // rather than against another implementation.
  if (/levenshtein\(/.test(expr)) {
    lines.push(
      "def levenshtein(a, b):",
      "    prev = list(range(len(b) + 1))",
      "    for i, ca in enumerate(a, 1):",
      "        cur = [i]",
      "        for j, cb in enumerate(b, 1):",
      "            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))",
      "        prev = cur",
      "    return prev[-1]",
      "");
  }
  return lines;
}

// The JavaScript pane is generated, then proved. `transpileJavaScript` gives the
// readable direct-API form; anything that still needs structural builders is not
// something to put beside a two-line Python snippet, so those cases get no tab.
// The generated code is then executed through the SDK and its decoded result
// compared with BioLang's, which is the same check `check-js-equivalence.mjs`
// runs in CI. A pane only reaches the page if that comparison passes.
const JS_PREAMBLE = ['import { BioLang } from "biolang";', "const bl = await BioLang.create();", ""];

function readableJavaScript(source) {
  const generated = bl.transpileJavaScript(source);
  if (generated.includes("bio.program(")) return null;
  return generated
    .split(NL)
    .filter((line) => !line.startsWith("// Direct JavaScript API;"))
    .join(NL)
    .trim();
}

async function javascriptPane(expr) {
  let expected;
  try {
    expected = bl.evalValue(expr);
  } catch {
    return null;           // needs I/O or runtime state we cannot reproduce here
  }
  const checkable = readableJavaScript(expr);
  const display = readableJavaScript(`println(${expr})`);
  if (!checkable || !display) return null;
  let actual;
  try {
    const body = checkable.replace(/\n([A-Za-z_$][\w$]*);\s*$/, "\nreturn $1;");
    actual = await new Function("bl", `return (async () => {\n${body}\n})();`)(bl);
  } catch {
    return null;
  }
  if (!isDeepStrictEqual(actual, expected)) return null;
  // The transpiler ends a program by binding the final value and echoing the
  // binding, which is how a REPL reports a result. A documentation pane wants
  // the call itself, so drop the echo and the binding it exists to name.
  const lines = display.split(NL);
  const echoed = lines.at(-1).trim().replace(/;$/, "");
  const statements = /^[A-Za-z_$][\w$]*$/.test(echoed)
    ? lines.slice(0, -1).map((line) =>
        line.startsWith(`let ${echoed} = `) ? line.slice(`let ${echoed} = `.length) : line)
    : lines;
  return JS_PREAMBLE.concat(statements).join(NL);
}

function rSetup(expr) {
  const lines = [];
  if (/DNAString|RNAString|reverseComplement|translate\(|complement\(/.test(expr)) {
    lines.push("library(Biostrings)");
  }
  return lines;
}


function paneCode(lang, expr) {
  const body = expr.trim();
  if (lang === "biolang") return `println(${body})`;
  // The JavaScript pane arrives ready to render: it was generated and executed
  // before it got here, so there is nothing left to wrap.
  if (lang === "javascript") return body;
  if (lang === "python") return pythonSetup(body).concat([`print(${body})`]).join(NL);
  return rSetup(body).concat([`print(${body})`]).join(NL);
}

const CATEGORY_TITLES = {
  bio: "Sequences",
  stats: "Statistics",
  math: "Maths",
  string: "Strings",
  list: "Lists",
  kmer: "K-mers",
};

function tabs(row, verdict) {
  const panes = [
    ["BioLang", "biolang", row.biolang],
    ["JavaScript", "javascript", row.javascript],
    ["Python", "python", row.python],
    ["R", "r", row.r],
  ].filter(([, , code]) => code && code.trim());

  const differ = (row.expect || "").trim() === "differ";
  const note = differ ? "conventions differ - see note" : "verified identical";
  // The evidence file lives in this repository and the page lives in another,
  // so the link has to leave the site rather than climb out of it.
  const href = differ ? "#conventions" : EVIDENCE_URL;

  const body = panes.map(([label, lang, code]) =>
    `        <div class="code-tab-pane" data-lang="${escape(label)}">
          <pre><code class="language-${lang}">${escape(paneCode(lang, code))}</code></pre>
        </div>`).join("\n");

  const values = verdict
    ? `        <p class="text-xs text-slate-500 mt-2">Returns <code>${escape(JSON.stringify(verdict.biolang))}</code>`
      + (verdict.python !== undefined
          ? ` in BioLang, <code>${escape(JSON.stringify(verdict.python))}</code> in Python`
          : "")
      + (verdict.r !== null && verdict.r !== undefined
          ? `, <code>${escape(JSON.stringify(verdict.r))}</code> in R`
          : "")
      + ".</p>"
    : "";

  return `      <div class="code-tabs" data-verified-note="${escape(note)}" data-verified-href="${escape(href)}">
${body}
      </div>
${values}`;
}

async function main() {
  const rows = loadCases();
  const results = loadResults();
  for (const row of rows) row.javascript = await javascriptPane(row.biolang);
  const withJavaScript = rows.filter((row) => row.javascript).length;
  const byCategory = new Map();
  for (const row of rows) {
    const key = row.category || "other";
    if (!byCategory.has(key)) byCategory.set(key, []);
    byCategory.get(key).push(row);
  }

  const sections = [];
  for (const [cat, items] of byCategory) {
    const title = CATEGORY_TITLES[cat] || cat;
    const blocks = items.map((row) => {
      const verdict = results?.get(row.name) ?? null;
      return `      <h3 class="text-base font-semibold text-white mt-8 mb-2">${escape(row.name)}</h3>\n`
        + tabs(row, verdict);
    }).join("\n");
    sections.push(`    <section class="mb-10">
      <h2 class="text-2xl font-bold text-white mb-2">${escape(title)}</h2>
${blocks}
    </section>`);
  }

  const differing = rows.filter((r) => (r.expect || "").trim() === "differ");
  const conventions = differing.length === 0 ? "" : `
    <section id="conventions" class="mb-10">
      <h2 class="text-2xl font-bold text-white mb-2">Where the conventions differ</h2>
      <p class="text-slate-400 mb-4">
        These are recorded rather than hidden. Neither answer is wrong; the
        languages round ties in different directions. The correctness suite
        fails if one of these ever starts agreeing, so the note cannot go stale.
      </p>
      <ul class="list-disc pl-6 text-slate-400 space-y-1">
${differing.map((r) => {
    const v = results?.get(r.name);
    const shown = v
      ? `BioLang <code>${escape(JSON.stringify(v.biolang))}</code>, `
        + `Python <code>${escape(JSON.stringify(v.python))}</code>`
        + (v.r != null ? `, R <code>${escape(JSON.stringify(v.r))}</code>` : "")
      : "";
    return `        <li><code>${escape(r.biolang)}</code> &mdash; ${shown}</li>`;
  }).join("\n")}
      </ul>
      <p class="text-slate-400 mt-4">
        BioLang rounds half away from zero. Python and R round half to even.
      </p>
    </section>`;

  const counted = rows.length;
  const html = `<!DOCTYPE html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8">
  <script>if(localStorage.getItem("theme")==="light")document.documentElement.classList.remove("dark")</script>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Verified Equivalents &mdash; BioLang</title>
  <meta name="description" content="The same computation in BioLang, JavaScript, Python and R, with every case checked rather than asserted.">
  <link rel="icon" href="../../assets/favicon.svg">
  <link rel="stylesheet" href="../../assets/styles.css">
  <link id="hljs-theme" rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
  <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
  <script src="../../js/biolang-highlight.js"></script>
  <style>
    .code-tab-strip { display: flex; align-items: center; gap: 0.25rem; margin-bottom: -1px; flex-wrap: wrap; }
    .code-tab { padding: 0.3rem 0.85rem; font-size: 0.8rem; font-weight: 600; color: rgb(148,163,184);
                background: rgba(30,41,59,0.6); border: 1px solid rgba(51,65,85,0.6); border-bottom: none;
                border-radius: 0.4rem 0.4rem 0 0; cursor: pointer; }
    .code-tab:hover { color: rgb(226,232,240); }
    .code-tab-active { color: rgb(167,139,250); background: rgb(15,23,42); border-color: rgba(139,92,246,0.4); }
    .code-tab-note { margin-left: auto; font-size: 0.7rem; color: rgb(100,116,139); text-decoration: none; }
    a.code-tab-note:hover { color: rgb(167,139,250); text-decoration: underline; }
    .code-tabs pre { margin-top: 0; border-top-left-radius: 0; }
    /* Without JavaScript every pane shows, stacked. Nothing is lost. */
    .code-tab-pane[hidden] { display: none; }
  </style>
</head>
<body class="bg-slate-950 text-slate-300">
  <!-- Generated by scripts/generate-equivalents.mjs from
       benchmarks/correctness/oneliners/cases.tsv. Do not edit by hand. -->
  <div data-component="header" data-base-path="../.."></div>
  <div class="flex max-w-[90rem] mx-auto">
    <div data-component="nav" data-base-path="../.." data-active="examples/equivalents"></div>
    <main class="flex-1 min-w-0 px-6 py-10 max-w-4xl">
      <h1 class="text-4xl font-bold text-white mb-3">Verified equivalents</h1>
      <p class="text-lg text-slate-400 mb-4">
        The same computation in BioLang, JavaScript, Python and R. Every case on
        this page is run and compared rather than eyeballed &mdash; floats to
        1e-9, integers and strings exactly &mdash; so these are checked
        translations rather than plausible ones.
      </p>
      <p class="text-slate-400 mb-8">
        ${counted} cases. The Python and R panes are written by hand and checked
        by the correctness suite. The JavaScript panes are not written at all:
        the shipped transpiler generates each one from the BioLang beside it, and
        a pane only appears if running it through the
        <a href="../tools/javascript.html" class="text-violet-400 hover:text-violet-300">JavaScript SDK</a>
        returns the same decoded value BioLang returns &mdash; ${withJavaScript}
        of ${counted} do.
        Only the BioLang tab has a Run button, because only BioLang runs in this
        page; every tab can be copied. Add a case by adding one row to
        <code>benchmarks/correctness/oneliners/cases.tsv</code>, and it appears
        here once it passes.
      </p>
${sections.join("\n")}
${conventions}
    </main>
  </div>
  <div data-component="footer" data-base-path="../.."></div>
  <script src="../../js/main.js"></script>
  <script src="../../js/copy-code.js"></script>
  <script src="../../js/code-tabs.js"></script>
  <script src="../../js/playground.js"></script>
</body>
</html>
`;

  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, html, "utf8");
  const differCount = differing.length;
  console.log(`equivalents: ${counted} cases across ${byCategory.size} categories `
    + `(${differCount} recorded as convention differences, `
    + `${withJavaScript} with a verified JavaScript pane) -> `
    + `${path.relative(ROOT, OUT).replace(/\\/g, "/")}`);
}

const bl = await BioLang.create({ network: false });
try {
  await main();
} finally {
  bl.dispose();
}
