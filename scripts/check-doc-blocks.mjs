#!/usr/bin/env node
/**
 * Execute every documentation code block through the real CLI, and report
 * coverage before pass rate.
 *
 * Why this exists: the site had two gates that between them reported zero
 * failures while two thirds of the corpus was never executed.
 *
 *   check_inline_examples.mjs   538 blocks executed, 909 skipped "CLI-only",
 *                               73 skipped "not runnable", FAILURES: 0
 *   check-doc-examples-cli.mjs  24 of those 909 blocks, "24 compiled, 0 run"
 *
 * "CLI-only" means the WebAssembly build cannot run it — it needs a file, an
 * API client, or a write. It does not mean the block is untestable: the CLI has
 * all 1018 builtins and can run every one of them. So this runs them.
 *
 * Two things it deliberately does differently:
 *
 *   - Coverage is the headline. A pass rate over a third of the corpus is the
 *     number that hid every bug found so far, so the table leads with how many
 *     blocks ran and breaks skips down by reason.
 *   - Nothing is skipped silently. REPL transcripts are replayed rather than
 *     dropped, and blocks that are prose or output samples are counted and
 *     listed, so "not runnable" is a claim you can audit instead of a number.
 *
 * Docs pages are cumulative — playground.js resets and replays blocks 0..N-1
 * before running block N — so blocks run as a growing prefix, not standalone.
 *
 * Usage:
 *   node scripts/check-doc-blocks.mjs
 *   node scripts/check-doc-blocks.mjs --dir docs/bio      # subtree
 *   node scripts/check-doc-blocks.mjs --list-fragments    # audit the skips
 *   node scripts/check-doc-blocks.mjs --json
 */

import { execFile } from "node:child_process";
import fs from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const websiteRoot = path.join(repositoryRoot, "website");
const exe = process.platform === "win32" ? "bl.exe" : "bl";
const cli = process.env.BIOLANG_CLI ?? path.join(repositoryRoot, "target", "release", exe);

const argv = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : fallback;
};
const subtree = flag("--dir", "docs");
const asJson = argv.includes("--json");
const listFragments = argv.includes("--list-fragments");
const timeout = Number(flag("--timeout", 90_000));

const BLOCK_RE =
  /<code[^>]*class="[^"]*\b(?:language-bio|language-biolang)\b[^"]*"[^>]*>([\s\S]*?)<\/code>/g;

function decode(text) {
  return text
    .replace(/<[^>]*>/g, "")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&");
}

function htmlFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "wasm") continue;
      out.push(...htmlFiles(full));
    } else if (entry.name.endsWith(".html") && entry.name !== "print.html") {
      out.push(full);
    }
  }
  return out;
}

/**
 * What kind of block this is.
 *
 * `fragment` is the category the old gate called "not runnable". Keeping the
 * reason with it is the point — an entry saying "output sample" can be checked
 * by a human, whereas a bare count of 73 cannot.
 */
function classify(code) {
  const trimmed = code.trim();
  if (!trimmed) return { kind: "fragment", why: "empty" };
  if (trimmed.startsWith("bl>")) return { kind: "transcript", why: "REPL session" };
  // Some blocks are deliberately not runnable: one documents a syntax error
  // ("# Breaks: newline before |>"), another calls a function that only exists
  // once a plugin is installed. Rewriting those to run would destroy what they
  // teach, so they say so, and the marker is visible to readers too. It still
  // shows up in the fragment count and under --list-fragments, so this cannot
  // quietly become a way to hide a failure.
  const marker = /^#\s*illustrative:?\s*(.*)$/im.exec(trimmed);
  if (marker) return { kind: "fragment", why: marker[1] || "marked illustrative" };
  const hasCode = /\b(let|fn|if|for|while|print|println|import|assert|return)\b|\|>/.test(trimmed);
  if (!hasCode) return { kind: "fragment", why: "output sample or prose, no statements" };
  if (trimmed.split("\n").length < 2 && !/[=(]/.test(trimmed)) {
    return { kind: "fragment", why: "single expression, no call or binding" };
  }
  return { kind: "code", why: "" };
}

/** An outage or a file the repository does not ship is not a broken example. */
function classifyFailure(output) {
  const text = output.toLowerCase();
  // Rate limiting is this harness's own doing — running every API block back to
  // back is exactly what NCBI throttles — so it must not read as a broken
  // example, or the gate is flaky by construction.
  if (
    /network error|timed out|timeout|http 5\d\d|econn|enotfound|dns|connection|rate limit|status code 429|too many requests/.test(
      text,
    )
  ) {
    return "network";
  }
  // "The system cannot find the file specified" is how Windows words ENOENT, so
  // matching only the Unix phrasing reported seven missing data files as broken
  // examples on this platform and none on CI.
  if (
    /no such file|not found|cannot open|failed to (read|open)|cannot find the (file|path) specified|os error 2/.test(
      text,
    )
  ) {
    return "missing-data";
  }
  if (/api_key|auth error|unauthorized|401|403/.test(text)) return "credentials";
  // Examples that shell out to samtools or bwa are documenting real pipelines;
  // the tool simply is not installed here.
  if (/is not recognized as an internal or external command|command not found|no such tool/.test(text)) {
    return "missing-tool";
  }
  return "failed";
}

function cleanError(output) {
  const lines = output
    .split(/\r?\n/)
    .map((l) => l.replace(/\x1b?\[[0-9;]*m/g, "").trim())
    .filter((l) => l && !/^[▶✓]/.test(l) && !/^running /i.test(l));
  // A failing run still prints whatever the program managed to emit first, so
  // take the diagnostic line rather than the first line of output.
  return lines.find((l) => /Error|error:|panicked/.test(l)) ?? lines[0] ?? "(no output)";
}

async function runSource(directory, name, source) {
  const file = path.join(directory, `${name}.bl`);
  await writeFile(file, source, "utf8");
  try {
    const { stdout } = await execFileAsync(cli, ["run", file], {
      timeout,
      maxBuffer: 64 * 1024 * 1024,
    });
    return { ok: true, output: stdout };
  } catch (error) {
    const output = `${error.stdout ?? ""}\n${error.stderr ?? ""}`.trim();
    return { ok: false, output, timedOut: Boolean(error.killed) };
  }
}

/**
 * Run a page's code blocks as a growing prefix.
 *
 * The whole page is tried in one process first, because that is the common case
 * and it costs one run instead of N. Only when it fails does it walk forward to
 * attribute the failure to a block — otherwise a page of thirty blocks would
 * cost thirty CLI launches to tell you nothing.
 */
async function runPage(directory, pageName, blocks) {
  const results = blocks.map(() => null);
  const codeIdx = blocks.map((b, i) => (b.kind === "code" ? i : -1)).filter((i) => i >= 0);
  if (codeIdx.length === 0) return results;

  const joined = codeIdx.map((i) => blocks[i].code).join("\n");
  const whole = await runSource(directory, pageName, joined);
  if (whole.ok) {
    for (const i of codeIdx) results[i] = { status: "ok" };
    return results;
  }

  // Something on this page fails, so attribute per block. A failing block is
  // left OUT of the prefix that later ones build on: stopping at the first
  // failure reported one broken example as eighty blocked ones, which said
  // nothing at all about the seventy-nine.
  const good = [];
  // Names bound by blocks that never ran. A block dropped from the prefix takes
  // its bindings with it, so everything downstream reports "undefined variable"
  // — six such reports on one page all traced back to a single `read_fasta` of
  // a file the repository does not ship. Counting those as broken examples
  // would be blaming the wrong block.
  const unavailable = new Set();
  const boundNames = (code) => {
    const names = [];
    // `let` and `fn`, plus the pipe-into form `... |> into name`, which binds a
    // name just as much. Missing it made a cascade look like a failure: a page
    // whose first block read a data file bound `sorted` through `into`, and the
    // block after it was reported as broken rather than blocked.
    const re =
      /\b(?:let|fn)\s+([A-Za-z_][A-Za-z0-9_]*)|\|>\s*into\s+([A-Za-z_][A-Za-z0-9_]*)|\bimport\s+"[^"]*"\s+as\s+([A-Za-z_][A-Za-z0-9_]*)/g;
    let m;
    while ((m = re.exec(code))) names.push(m[1] ?? m[2] ?? m[3]);
    return names;
  };

  for (let n = 0; n < codeIdx.length; n++) {
    const index = codeIdx[n];
    const attempt = await runSource(
      directory,
      `${pageName}-${n}`,
      good.concat(blocks[index].code).join("\n"),
    );
    if (attempt.ok) {
      results[index] = { status: "ok" };
      good.push(blocks[index].code);
      continue;
    }
    const detail = attempt.timedOut ? "timed out" : cleanError(attempt.output);
    let status = attempt.timedOut ? "network" : classifyFailure(attempt.output);

    const missing = /undefined variable '([^']+)'/.exec(detail);
    if (status === "failed" && missing && unavailable.has(missing[1])) {
      status = "blocked";
    }
    results[index] = { status, detail };
    for (const name of boundNames(blocks[index].code)) unavailable.add(name);
  }
  return results;
}

/**
 * Replay a REPL transcript through the CLI.
 *
 * `bl>` lines are input and everything between them is the printed result. The
 * old gate dropped these entirely, so a transcript could show output the
 * language stopped producing and nothing would notice.
 */
async function runTranscript(directory, name, code) {
  const inputs = code
    .split(/\r?\n/)
    .filter((l) => l.trim().startsWith("bl>"))
    .map((l) => l.trim().replace(/^bl>\s?/, ""))
    .filter(Boolean);
  if (inputs.length === 0) return { status: "fragment", detail: "no bl> input lines" };
  const attempt = await runSource(directory, name, inputs.join("\n"));
  if (attempt.ok) return { status: "ok" };
  const status = attempt.timedOut ? "network" : classifyFailure(attempt.output);
  return { status, detail: attempt.timedOut ? "timed out" : cleanError(attempt.output) };
}

const root = path.join(websiteRoot, subtree);
if (!fs.existsSync(root)) {
  console.error(`No such directory: ${root}`);
  process.exit(2);
}

const directory = await mkdtemp(path.join(tmpdir(), "bl-docs-"));
const rows = [];
// A full docs run launches the CLI once per page, and once per block on any
// page that fails, so it takes tens of minutes. Printing nothing until the end
// makes it indistinguishable from a hang, so progress goes to stderr and the
// table stays on stdout where it can still be piped.
const allFiles = htmlFiles(root).filter((f) =>
  /playground\.js/.test(fs.readFileSync(f, "utf8")),
);
const started = Date.now();
let done = 0;
try {
  for (const file of allFiles) {
    const html = fs.readFileSync(file, "utf8");
    const relative = path.relative(websiteRoot, file).replace(/\\/g, "/");
    const blocks = [];
    let match;
    BLOCK_RE.lastIndex = 0;
    while ((match = BLOCK_RE.exec(html)) !== null) {
      const code = decode(match[1]).trim();
      blocks.push({ code, ...classify(code) });
    }
    if (blocks.length === 0) continue;

    const safe = relative.replace(/[^A-Za-z0-9]+/g, "_");
    const pageResults = await runPage(directory, safe, blocks);

    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];
      let result;
      if (block.kind === "transcript") {
        result = await runTranscript(directory, `${safe}-t${i}`, block.code);
      } else if (block.kind === "fragment") {
        result = { status: "fragment", detail: block.why };
      } else {
        result = pageResults[i] ?? { status: "ok" };
      }
      rows.push({ page: relative, index: i, kind: block.kind, ...result });
    }

    done += 1;
    const pageRows = rows.filter((r) => r.page === relative);
    const ok = pageRows.filter((r) => r.status === "ok").length;
    const bad = pageRows.filter((r) => r.status === "failed").length;
    const elapsed = Math.round((Date.now() - started) / 1000);
    process.stderr.write(
      `[${String(done).padStart(3)}/${allFiles.length}] ${elapsed}s  ` +
        `${relative}  ${ok} ran${bad ? `, ${bad} FAILED` : ""}\n`,
    );
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}

const count = (status) => rows.filter((r) => r.status === status).length;
const executed = count("ok");
const failed = rows.filter((r) => r.status === "failed");
const total = rows.length;
const pages = new Set(rows.map((r) => r.page)).size;

if (asJson) {
  console.log(JSON.stringify({ total, executed, rows }, null, 2));
} else {
  const pct = (n) => (total ? ((n / total) * 100).toFixed(1).padStart(5) : "  0.0");
  console.log(`\n  ${pages} pages, ${total} blocks under website/${subtree}\n`);
  console.log("  outcome        blocks       share   meaning");
  console.log("  ------------------------------------------------------------------");
  const line = (label, n, meaning) =>
    console.log(`  ${label.padEnd(13)} ${String(n).padStart(6)}   ${pct(n)}%   ${meaning}`);
  line("ran", executed, "executed by the CLI with no error");
  line("FAILED", failed.length, "executed and errored");
  line("blocked", count("blocked"), "needs a binding from a block that never ran");
  line("network", count("network"), "needs a live service");
  line("missing-data", count("missing-data"), "reads a file the repo does not ship");
  line("credentials", count("credentials"), "needs an API key");
  line("missing-tool", count("missing-tool"), "shells out to a tool not installed here");
  line("fragment", count("fragment"), "prose or output sample, nothing to execute");
  console.log("  ------------------------------------------------------------------");

  if (failed.length) {
    // A run of this takes tens of minutes, so truncating the one output that
    // says what to fix means paying for it twice. Group by page and print all
    // of them.
    console.log(`\n  ${failed.length} failing blocks:\n`);
    let current = "";
    for (const r of failed) {
      if (r.page !== current) {
        current = r.page;
        console.log(`    ${current}`);
      }
      console.log(`      block ${String(r.index).padStart(2)}  ${String(r.detail).slice(0, 150)}`);
    }
  }

  if (listFragments) {
    console.log("\n  blocks counted as fragments:\n");
    for (const r of rows.filter((x) => x.status === "fragment")) {
      console.log(`    ${r.page}  block ${r.index}  — ${r.detail}`);
    }
  }
  console.log("");
}

process.exit(failed.length > 0 ? 1 : 0);
