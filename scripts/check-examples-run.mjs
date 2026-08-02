#!/usr/bin/env node
/**
 * Run every example and hold the line on the ones that are broken.
 *
 * Background: 22 of 58 runnable examples failed, including `quickstart.bl` —
 * the first thing a new user runs. Nothing caught it because the only runtime
 * gate was a hardcoded list of 21 known-good paths, so a newly-added or newly-
 * broken example was invisible by default.
 *
 * A plain "everything must pass" gate cannot be turned on until all of them are
 * fixed, and a gate that is off until some future cleanup protects nothing. So
 * this uses a ratchet instead:
 *
 *   - an example that fails and is NOT listed in known-broken.txt fails the run
 *     (no new breakage)
 *   - an example that passes but IS listed fails the run, telling you to delete
 *     the line (the list can only shrink)
 *
 * `examples/apis/` and `examples/research/` are excluded: they need network
 * access and credentials, so they cannot be a build gate.
 *
 * Usage: node scripts/check-examples-run.mjs [--timeout 60000]
 */

import { execFile } from "node:child_process";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const examplesRoot = path.join(repositoryRoot, "examples");
const knownBrokenPath = path.join(examplesRoot, "known-broken.txt");
const cli = process.env.BIOLANG_CLI
  ?? path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");

const timeoutIndex = process.argv.indexOf("--timeout");
const timeout = timeoutIndex >= 0 ? Number(process.argv[timeoutIndex + 1]) : 60_000;

// Directories that reach the network or need credentials.
const EXCLUDED = new Set(["apis", "research"]);

async function examples(directory, relativeTo = examplesRoot) {
  const found = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    const relative = path.relative(relativeTo, absolute).replaceAll("\\", "/");
    if (entry.isDirectory()) {
      if (EXCLUDED.has(relative)) continue;
      found.push(...(await examples(absolute, relativeTo)));
    } else if (entry.name.endsWith(".bl")) {
      found.push(relative);
    }
  }
  return found.sort();
}

const knownBroken = new Set(
  (await readFile(knownBrokenPath, "utf8").catch(() => ""))
    .split(/\r?\n/)
    .map((line) => line.replace(/#.*$/, "").trim())
    .filter(Boolean),
);

const files = await examples(examplesRoot);
const failing = [];
const passing = [];
const networkFailures = [];

/**
 * Examples that call a remote service declare it with `# requires: network`.
 *
 * A live service returning 503 is not a BioLang regression, and a gate that
 * goes red for it teaches people to ignore red. These still run — a failure is
 * reported — but they cannot fail the build. The marker lives in the file so
 * there is no second list to drift out of sync.
 */
const needsNetwork = new Set();
for (const relative of files) {
  const source = await readFile(path.join(examplesRoot, relative), "utf8");
  if (/^#\s*requires:\s*network\b/m.test(source)) needsNetwork.add(relative);
}

for (const relative of files) {
  try {
    await execFileAsync(cli, ["run", path.join("examples", relative)], {
      cwd: repositoryRoot,
      timeout,
      maxBuffer: 16 * 1024 * 1024,
      env: { ...process.env, BIOLANG_NO_UPDATE_CHECK: "1" },
    });
    passing.push(relative);
  } catch (error) {
    const message = String(error.stderr || error.stdout || error.message)
      .split(/\r?\n/)
      .find((line) => /Error|error:/.test(line)) ?? "failed";
    if (needsNetwork.has(relative)) networkFailures.push({ relative, message: message.trim() });
    else failing.push({ relative, message: message.trim() });
  }
}

const regressions = failing.filter((entry) => !knownBroken.has(entry.relative));
const fixed = passing.filter((relative) => knownBroken.has(relative));
const stale = [...knownBroken].filter((relative) => !files.includes(relative));

console.log(
  `${passing.length} of ${files.length} examples run ` +
    `(${knownBroken.size} known broken, ${regressions.length} new failures, ` +
    `${needsNetwork.size} need the network)`,
);

if (networkFailures.length > 0) {
  console.warn(`\n${networkFailures.length} network example(s) failed — not counted as a regression:`);
  for (const entry of networkFailures) console.warn(`  ${entry.relative}: ${entry.message}`);
}

if (regressions.length > 0) {
  console.error(`\n${regressions.length} example(s) newly broken:`);
  for (const entry of regressions) console.error(`  ${entry.relative}: ${entry.message}`);
}
if (fixed.length > 0) {
  console.error(`\n${fixed.length} example(s) now pass — remove them from examples/known-broken.txt:`);
  for (const relative of fixed) console.error(`  ${relative}`);
}
if (stale.length > 0) {
  console.error(`\n${stale.length} entr(y/ies) in known-broken.txt no longer exist:`);
  for (const relative of stale) console.error(`  ${relative}`);
}

if (regressions.length || fixed.length || stale.length) process.exit(1);
