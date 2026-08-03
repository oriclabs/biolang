#!/usr/bin/env node
/**
 * Pre-publish gate for the website. NOT part of the build, and not something to
 * run on every commit — this is the check you run before shipping the site,
 * because it exercises every runnable example in a real browser and takes on the
 * order of an hour.
 *
 *   npm run prepublish-check                  # everything
 *   npm run prepublish-check -- --quick       # static checks only (seconds)
 *   npm run prepublish-check -- --dir docs/bio
 *
 * Stages, cheapest first, so an obvious break fails fast:
 *
 *   1  code-block HTML       markup that stops a block being runnable at all
 *   2  unsupported syntax    constructs the language does not have
 *   3  string interpolation  placeholders accidentally left in plain strings
 *   4  builtin call arity    calls that cannot match the real signature
 *   5  browser run-buttons   Playwright: click every Run button, check output
 *
 * Stages 1-3 are static and fast. Stage 4 needs the WASM bundle in website/wasm
 * to be current — rebuild it first if the runtime changed:
 *
 *   wasm-pack build crates/bl-wasm --target web --out-dir ../../website/wasm \
 *     --no-typescript --release
 *
 * Exit code is 0 only if every enabled stage passes.
 */
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SITE = path.resolve(HERE, "..");
const REPO = path.resolve(SITE, "..");

const argv = process.argv.slice(2);
const QUICK = argv.includes("--quick");
const pick = (f) => (argv.indexOf(f) >= 0 ? argv[argv.indexOf(f) + 1] : null);
const DIR = pick("--dir");

const BOLD = "\x1b[1m", DIM = "\x1b[2m", RED = "\x1b[31m", GRN = "\x1b[32m",
      YEL = "\x1b[33m", OFF = "\x1b[0m";

/** Metadata drives the arity check; regenerate so it matches this build. */
function ensureMetadata() {
  const out = path.join(SITE, "tests", "builtin-metadata.json");
  process.stdout.write(`${DIM}  regenerating builtin metadata...${OFF}`);
  const r = spawnSync(
    "cargo",
    ["run", "-q", "-p", "bl-cli", "--", "metadata", "--format", "json"],
    { cwd: REPO, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 }
  );
  if (r.status !== 0 || !r.stdout || !r.stdout.trim().startsWith("{")) {
    console.log(`\r${YEL}  ! could not regenerate metadata${OFF}`);
    return fs.existsSync(out) ? out : null;
  }
  fs.writeFileSync(out, r.stdout);
  console.log(`\r${DIM}  builtin metadata written to tests/builtin-metadata.json${OFF}`);
  return out;
}

const stages = [];

if (fs.existsSync(path.join(HERE, "check_code_block_html.mjs"))) {
  stages.push({
    name: "code-block HTML",
    why: "markup that prevents a block being runnable",
    cmd: ["node", ["tests/check_code_block_html.mjs", ...(DIR ? ["--dir", DIR] : [])]],
  });
}

stages.push({
  name: "unsupported syntax",
  why: "constructs the language does not have (';', '.0', 'for a, b in', 'f(x = 1)')",
  cmd: ["node", ["tests/check_doc_syntax.mjs", ...(DIR ? ["--dir", DIR] : [])]],
});

stages.push({
  name: "string interpolation",
  why: "placeholders that would print literally because an f-string prefix is missing",
  cmd: ["node", ["tests/check_interpolation.mjs", DIR ?? "docs"]],
});

const META = ensureMetadata();
if (META) {
  stages.push({
    name: "builtin call arity",
    why: "calls that cannot satisfy the documented signature",
    // Known to over-report: see the header of check_call_signatures.mjs.
    advisory: true,
    cmd: ["node", ["tests/check_call_signatures.mjs", META, ...(DIR ? ["--dir", DIR] : [])]],
  });
}

if (!QUICK) {
  stages.push({
    name: "browser run-buttons",
    why: "clicks every Run button in Chromium and checks the output",
    slow: true,
    cmd: ["npx", ["playwright", "test"]],
    env: DIR ? { BL_DIR: DIR } : {},
  });
}

console.log(`${BOLD}website pre-publish check${OFF}`);
console.log(`${DIM}${QUICK ? "static stages only (--quick)" : "full run — the browser stage takes ~1h for the whole site"}${OFF}`);
if (DIR) console.log(`${DIM}scope: ${DIR}${OFF}`);
console.log("");

const results = [];
for (const st of stages) {
  const label = `${BOLD}${st.name}${OFF}${st.advisory ? ` ${YEL}(advisory)${OFF}` : ""}`;
  console.log(`── ${label}`);
  console.log(`   ${DIM}${st.why}${OFF}`);
  if (st.slow) console.log(`   ${DIM}this is the slow one — output streams below${OFF}`);
  const started = Date.now();
  const r = spawnSync(st.cmd[0], st.cmd[1], {
    cwd: SITE,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: { ...process.env, ...(st.env || {}) },
  });
  const secs = ((Date.now() - started) / 1000).toFixed(0);
  const ok = r.status === 0;
  results.push({ name: st.name, ok, advisory: !!st.advisory, secs });
  console.log(
    `   ${ok ? GRN + "pass" : (st.advisory ? YEL + "findings" : RED + "FAIL")}${OFF}` +
      ` ${DIM}(${secs}s)${OFF}\n`
  );
}

console.log(`${BOLD}summary${OFF}`);
let blocking = 0;
for (const r of results) {
  const tag = r.ok ? `${GRN}pass    ${OFF}`
    : r.advisory ? `${YEL}findings${OFF}`
    : `${RED}FAIL    ${OFF}`;
  if (!r.ok && !r.advisory) blocking++;
  console.log(`  ${tag}  ${r.name.padEnd(22)} ${DIM}${r.secs}s${OFF}`);
}

if (blocking === 0) {
  console.log(`\n${GRN}ready to publish${OFF}` +
    (results.some((r) => !r.ok) ? ` ${DIM}(advisory findings above — review, not blocking)${OFF}` : ""));
} else {
  console.log(`\n${RED}${blocking} blocking stage(s) failed — do not publish${OFF}`);
  if (!QUICK) console.log(`${DIM}browser failures: npm run prepublish-check:report${OFF}`);
}
process.exit(blocking === 0 ? 0 : 1);
