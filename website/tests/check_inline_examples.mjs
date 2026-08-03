/**
 * Execute every runnable inline example on the site through the same WASM
 * module the browser uses, and report the ones that fail.
 *
 * The site has TWO runners with different semantics, and each page is checked
 * under whichever one it actually loads:
 *
 *   book-runner   (books/, 106 pages) — CUMULATIVE: block N runs after blocks
 *       0..N-1 with shared interpreter state; reset() once per page.
 *   playground.js (docs/)             — also cumulative: it resets then replays
 *       blocks 0..N-1, plus a stricter "does this contain code at all" filter.
 *
 * Pages loading neither runner have no Run buttons and are not checked. Each
 * runner's block selector, skip rules and CLI-only rules are ported from the
 * runner source so this measures what a reader actually sees.
 *
 * Usage:  node tests/check_inline_examples.mjs [--dir <subdir>] [--verbose]
 *                                              [--json <out.json>]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const VERBOSE = argv.includes("--verbose");
const arg = (f) => (argv.indexOf(f) >= 0 ? argv[argv.indexOf(f) + 1] : null);
const ONLY = arg("--dir");
const JSON_OUT = arg("--json");

// ── CLI-only rules, ported from each runner ─────────────────────────────────
const SHARED_CLI = [
  /\b(write_csv|write_tsv|write_json|write_text|write_lines|write_fasta|write_fastq|write_vcf|write_bed)\b/,
  /\btrim_quality\s*\(\s*["']/,
  /\b(open|save|write_file|write_lines|mkdir)\s*\(/,
  /\b(save_plot|save_svg|save_png)\s*\(/,
  /\b(read_sam|read_bam)\b/,
  /\b(ensembl_gene|ensembl_vep|uniprot_search|uniprot_entry|kegg_get|kegg_find|pdb_entry|string_network|go_term|go_annotations|cosmic_gene|datasets_gene|reactome_pathways|ucsc_sequence|fetch|http_get|http_post)\b/,
  /\b(chat|chat_code|llm|ask_llm)\s*\(/,
];
const cliBook = (t) =>
  SHARED_CLI.some((r) => r.test(t)) || /\b(notebook|pipeline|import\s+")\b/.test(t);
const cliPlayground = (t) =>
  SHARED_CLI.some((r) => r.test(t)) ||
  /\b(notebook|import\s+")\b/.test(t) ||
  /^\s*pipeline\s+\w/m.test(t);

// ── skip rules, ported from each runner ─────────────────────────────────────
function skipBook(t) {
  if (t.indexOf("bl>") === 0) return true;
  const lines = t.trim().split("\n");
  return lines.length < 2 && !t.includes("let ") && !t.includes("print") && !t.includes("|>");
}
function skipPlayground(t) {
  if (t.trimStart().indexOf("bl>") === 0) return true;
  const lines = t.trim().split("\n");
  if (
    lines.length < 2 &&
    !t.includes("let ") && !t.includes("print") && !t.includes("|>") && !t.includes("println")
  )
    return true;
  return !/\b(let|fn|if|for|while|print|println|dna"|rna"|protein"|import|\|>)\b/.test(t);
}

const RUNNERS = {
  book: {
    src: '<code[^>]*class="[^"]*\\b(?:language-bio|language-biolang|language-biorun)\\b[^"]*"[^>]*>([\\s\\S]*?)</code>',
    skip: skipBook,
    cli: cliBook,
    cumulative: true,
  },
  playground: {
    src: '<code[^>]*class="[^"]*\\b(?:language-bio|language-biolang)\\b[^"]*"[^>]*>([\\s\\S]*?)</code>',
    skip: skipPlayground,
    cli: cliPlayground,
    cumulative: true, // playground.js replays earlier blocks, same as book-runner
  },
};

const runnerFor = (html) =>
  /book-runner/.test(html) ? "book" : /playground\.js/.test(html) ? "playground" : null;

// ── HTML → code blocks ──────────────────────────────────────────────────────
const E = {
  "&lt;": "<", "&gt;": ">", "&amp;": "&", "&quot;": '"',
  "&#39;": "'", "&#x27;": "'", "&nbsp;": " ", "&#96;": "`", "&#x60;": "`",
};
const decode = (s) =>
  s
    .replace(/&(?:lt|gt|amp|quot|nbsp);|&#(?:x27|39|96|x60);/g, (m) => E[m] ?? m)
    .replace(/&#(\d+);/g, (_, d) => String.fromCharCode(+d))
    .replace(/&#x([0-9a-f]+);/gi, (_, h) => String.fromCharCode(parseInt(h, 16)));

function extractBlocks(html, runner) {
  const out = [];
  const re = new RegExp(RUNNERS[runner].src, "g");
  let m;
  while ((m = re.exec(html)) !== null) {
    // both runners treat a preceding "Requires CLI" note as CLI-only
    const before = html.slice(Math.max(0, m.index - 400), m.index);
    out.push({ code: decode(m[1].replace(/<[^>]*>/g, "")), noteCli: /Requires CLI/.test(before) });
  }
  return out;
}

function htmlFiles(dir) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === "node_modules" || e.name === "wasm") continue;
      out.push(...htmlFiles(p));
    } else if (e.name.endsWith(".html") && e.name !== "print.html") out.push(p);
  }
  return out;
}

// ── fetch bridge: the runner serves data over HTTP; here, from disk ─────────
const DATA_DIRS = [path.join(ROOT, "books", "data")];
globalThis.window = globalThis;
globalThis.__blFiles = {};
globalThis.__blFetch = {
  sync(url) {
    // Real network calls (NCBI etc.) do work in a browser but not offline here;
    // flag the block so it is reported separately, not counted as broken.
    if (/^https?:\/\//.test(url)) {
      globalThis.__usedNetwork = true;
      return "ERROR:404 offline harness";
    }
    if (globalThis.__blFiles[url]) return globalThis.__blFiles[url];
    const rel = url.replace(/^data\//, "");
    for (const c of [
      ...(globalThis.__pageDir ? [path.join(globalThis.__pageDir, url)] : []),
      ...DATA_DIRS.map((d) => path.join(d, rel)),
      ...DATA_DIRS.map((d) => path.join(d, url)),
    ]) {
      try {
        if (fs.existsSync(c)) return (globalThis.__blFiles[url] = fs.readFileSync(c, "utf8"));
      } catch {}
    }
    return "ERROR:404 File not found (" + url + ")";
  },
};

// ── WASM lifecycle ──────────────────────────────────────────────────────────
const WASM_BYTES = fs.readFileSync(path.join(ROOT, "wasm", "bl_wasm_bg.wasm"));

// A Rust panic inside evaluate() leaves the interpreter's RefCell borrowed, so
// every later call traps. Only a brand-new instance clears it — the browser
// equivalent is reloading the page. wasm-bindgen's init early-returns once
// initialised, and the ES module cache is URL-keyed, so a cache-busting query
// is what actually yields fresh thread-local state.
let wasm = null;
let gen = 0;
async function freshWasm() {
  const mod = await import(`../wasm/bl_wasm.js?gen=${gen++}`);
  await mod.default({ module_or_path: WASM_BYTES });
  mod.init();
  wasm = mod;
}

let lastPanic = null;
const realError = console.error;
console.error = (...a) => {
  const s = a.map(String).join(" ");
  if (lastPanic === null && /panicked/.test(s)) lastPanic = s.split("\n")[0].trim();
  if (VERBOSE) realError(...a);
};

await freshWasm();
const browserBuiltins = new Set(
  JSON.parse(wasm.list_builtins()).map((entry) => entry.name).filter(Boolean)
);
const browserDataFiles = new Set(
  fs.readdirSync(path.join(ROOT, "books", "data"), { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
);
const BL_KEYWORDS = new Set([
  "if", "for", "while", "fn", "let", "match", "return", "and", "or", "not",
  "in", "else", "import", "yield", "enum", "into", "true", "false", "nil",
]);

function localNames(code) {
  const names = new Set();
  for (const match of code.matchAll(/\b(?:let|fn)\s+([A-Za-z_][A-Za-z0-9_]*)/g))
    names.add(match[1]);
  for (const match of code.matchAll(/\bfn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(([^)]*)\)/g))
    for (const param of match[1].split(",")) {
      const name = param.split("=")[0].trim();
      if (name) names.add(name);
    }
  for (const match of code.matchAll(/\bfor\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\b/g))
    names.add(match[1]);
  for (const match of code.matchAll(/\|([^|\n]*)\|/g))
    for (const param of match[1].split(",")) {
      const name = param.trim();
      if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) names.add(name);
    }
  return names;
}

function topLevelNames(code) {
  const names = new Set();
  for (const match of code.matchAll(/(?:^|\n)\s*(?:let|fn)\s+([A-Za-z_][A-Za-z0-9_]*)/g))
    names.add(match[1]);
  return names;
}

function usesUnavailablePageLocal(code, unavailable) {
  const locals = localNames(code);
  for (const name of unavailable) {
    if (!locals.has(name) && new RegExp(`\\b${name}\\b`).test(code)) return true;
  }
  return false;
}

function hasUnavailableDataFile(code) {
  const re = /\b(?:read_csv|csv|tsv|read_fasta|fasta|read_fastq|fastq|read_vcf|vcf|read_bed|bed|read_gff|gff)\s*\(\s*/g;
  let match;
  while ((match = re.exec(code))) {
    const literal = code.slice(re.lastIndex).match(/^"([^"]+)"/);
    if (!literal) return true;
    const normalized = literal[1].replaceAll("\\", "/").replace(/^data\//, "");
    if (!browserDataFiles.has(normalized)) return true;
  }
  return false;
}

function unsupportedCalls(code, pageLocals) {
  const locals = localNames(code);
  const missing = new Set();
  for (const match of code.matchAll(/([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)) {
    const name = match[1];
    if (!locals.has(name) && !pageLocals.has(name) && !browserBuiltins.has(name) && !BL_KEYWORDS.has(name))
      missing.add(name);
  }
  return [...missing];
}

// ── run ─────────────────────────────────────────────────────────────────────
const roots = ONLY ? [path.join(ROOT, ONLY)] : [path.join(ROOT, "books"), path.join(ROOT, "docs")];
const stats = { pages: 0, ran: 0, cli: 0, skipped: 0, network: 0, noRunner: 0 };
const failures = [];
const slow = [];

for (const root of roots) {
  if (!fs.existsSync(root)) continue;
  for (const file of htmlFiles(root)) {
    const html = fs.readFileSync(file, "utf8");
    const runner = runnerFor(html);
    if (!runner) { stats.noRunner++; continue; }

    const blocks = extractBlocks(html, runner);
    if (blocks.length === 0) continue;
    const R = RUNNERS[runner];
    const availablePageLocals = new Set();
    const unavailablePageLocals = new Set();
    stats.pages++;
    globalThis.__pageDir = path.dirname(file);
    process.stderr.write(`[${stats.pages}] ${path.relative(ROOT, file)}
`);

    if (R.cumulative) {
      try { wasm.reset(); } catch { await freshWasm(); }
    }

    let idx = 0;
    for (const b of blocks) {
      if (R.skip(b.code)) { stats.skipped++; continue; }
      idx++;
      const cliOnly =
        b.noteCli ||
        R.cli(b.code) ||
        unsupportedCalls(b.code, availablePageLocals).length ||
        hasUnavailableDataFile(b.code) ||
        usesUnavailablePageLocal(b.code, unavailablePageLocals);
      if (cliOnly) {
        for (const name of topLevelNames(b.code)) unavailablePageLocals.add(name);
        stats.cli++;
        continue;
      }
      for (const name of topLevelNames(b.code)) availablePageLocals.add(name);
      if (!R.cumulative) {
        try { wasm.reset(); } catch { await freshWasm(); }
      }
      stats.ran++;
      const t0 = Date.now();
      lastPanic = null;
      globalThis.__usedNetwork = false;
      let res;
      try {
        res = JSON.parse(wasm.evaluate(b.code));
      } catch (e) {
        failures.push({
          file, idx, runner, code: b.code,
          error: "PANIC: " + (lastPanic || e.message), panic: true,
        });
        await freshWasm();
        continue;
      }
      const ms = Date.now() - t0;
      if (ms > 2000) {
        slow.push({ file: path.relative(ROOT, file), idx, ms });
        process.stderr.write(`    slow: #${idx} took ${ms}ms
`);
      }
      const err = res && (res.error || (res.ok === false ? res.output : null));
      if (err && globalThis.__usedNetwork) stats.network++;
      else if (err) failures.push({ file, idx, runner, code: b.code, error: String(err) });
      else if (VERBOSE) console.log(`ok  ${path.relative(ROOT, file)} #${idx}`);
    }
  }
}

console.log("");
console.log(`pages with a runner   : ${stats.pages}   (no runner, skipped: ${stats.noRunner})`);
console.log(`blocks executed       : ${stats.ran}`);
console.log(`skipped (CLI-only)    : ${stats.cli}`);
console.log(`skipped (not runnable): ${stats.skipped}`);
console.log(`network-dependent     : ${stats.network}  (work in a browser, not offline)`);
console.log(
  `FAILURES              : ${failures.length}` +
    (stats.ran ? `  (${((failures.length / stats.ran) * 100).toFixed(1)}%)` : "")
);

const cls = new Map();
for (const f of failures) {
  let k = f.panic ? "PANIC (poisons the page)" : f.error.split(/[:;\n]/)[0].trim().slice(0, 55);
  if (/undefined variable/.test(f.error)) k = "undefined variable";
  else if (/^expected /.test(f.error)) k = "parse error";
  else if (/no field/.test(f.error)) k = "no field on record";
  else if (/not supported on this platform/.test(f.error)) k = "unsupported in WASM";
  else if (/File not found|cannot open/.test(f.error)) k = "missing data file";
  else if (/requires|expected .* got|must be/i.test(f.error)) k = "type / arity error";
  cls.set(k, (cls.get(k) || 0) + 1);
}
if (cls.size) {
  console.log("\nby error class:");
  for (const [k, v] of [...cls].sort((a, b) => b[1] - a[1]))
    console.log(`  ${String(v).padStart(4)}  ${k}`);
}

const byFile = new Map();
for (const f of failures) {
  const k = path.relative(ROOT, f.file);
  byFile.set(k, (byFile.get(k) || 0) + 1);
}
if (byFile.size) {
  console.log("\nworst pages:");
  for (const [k, v] of [...byFile].sort((a, b) => b[1] - a[1]).slice(0, 12))
    console.log(`  ${String(v).padStart(4)}  ${k}`);
}

if (slow.length) {
  console.log(`
slowest blocks (>2s in the interpreter — these stall the browser tab):`);
  for (const s_ of slow.sort((a, b) => b.ms - a.ms).slice(0, 10))
    console.log(`  ${String(s_.ms).padStart(6)}ms  ${s_.file} #${s_.idx}`);
}

if (JSON_OUT) {
  fs.writeFileSync(
    JSON_OUT,
    JSON.stringify(
      { stats, failures: failures.map((f) => ({ ...f, file: path.relative(ROOT, f.file) })) },
      null,
      1
    )
  );
  console.log(`\nfull detail → ${JSON_OUT}`);
}
process.exit(failures.length ? 1 : 0);
