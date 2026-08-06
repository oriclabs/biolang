#!/usr/bin/env node
/**
 * Run every script in the MSMB companion and fail if any of them does.
 *
 * The companion quotes a lot of numbers - `dmultinom` to ten decimals, the
 * C. elegans goodness of fit, Hardy-Weinberg expected counts, sample sizes -
 * and several are checked against the values Holmes & Huber publish. That is
 * the book's main claim to being correct, and until this existed nothing
 * defended it: the scripts had been verified once, by hand, by watching the
 * output scroll past.
 *
 * A change to `gamma_cdf`, `pca`, `upgma` or the multinomial sampler would have
 * moved those figures silently, leaving prose that quotes numbers the code no
 * longer produces. The scripts now assert their published values, and this
 * runner is what makes those assertions fire.
 *
 * Every script is seeded, so the numbers are deterministic. A failure here means
 * either a real regression or a figure that legitimately moved - in which case
 * the prose needs updating too, which is the point.
 *
 * Usage: node scripts/check-msmb.mjs [--timeout 300000]
 */

import { spawnSync } from "node:child_process";
import { readdirSync, existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const codeRoot = path.join(repositoryRoot, "books", "msmb", "code");
const cli =
  process.env.BIOLANG_CLI ??
  path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");

const timeoutIndex = process.argv.indexOf("--timeout");
const timeout = timeoutIndex >= 0 ? Number(process.argv[timeoutIndex + 1]) : 300_000;

if (!existsSync(cli)) {
  console.error(`Cannot run the MSMB scripts: ${cli} is not built.`);
  console.error("Run `cargo build -p bl-cli`, or set BIOLANG_CLI.");
  process.exit(1);
}

/** Chapter directories, plus the shared package, in reading order. */
function scriptDirectories() {
  const dirs = readdirSync(codeRoot)
    .filter((name) => /^ch\d+$/.test(name))
    .sort();
  const shared = path.join("packages", "msmbstats", "src");
  if (existsSync(path.join(codeRoot, shared))) dirs.push(shared);
  return dirs;
}

let run = 0;
const failures = [];

for (const dir of scriptDirectories()) {
  const absolute = path.join(codeRoot, dir);
  if (!statSync(absolute).isDirectory()) continue;

  // Scripts use relative paths for imports and for the figures they write, so
  // each one runs from its own directory rather than from the repository root.
  for (const file of readdirSync(absolute).filter((f) => f.endsWith(".bl")).sort()) {
    // mod.bl is a package manifest comment, not a runnable script.
    if (file === "mod.bl") continue;

    run += 1;
    const result = spawnSync(cli, ["run", file], {
      cwd: absolute,
      timeout,
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
    });

    const label = `${dir}/${file}`;
    if (result.error?.code === "ETIMEDOUT") {
      failures.push({ label, why: `timed out after ${timeout} ms` });
    } else if (result.status !== 0) {
      const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
      // The assertion message is the useful part; the surrounding narration is
      // not, so keep the tail rather than the whole transcript.
      const tail = output.trimEnd().split("\n").slice(-12).join("\n");
      failures.push({ label, why: tail || `exited ${result.status}` });
    }
  }
}

if (failures.length) {
  console.error(`\n${failures.length} of ${run} MSMB scripts failed:\n`);
  for (const failure of failures) {
    console.error(`  ${failure.label}`);
    for (const line of failure.why.split("\n")) console.error(`      ${line}`);
    console.error("");
  }
  console.error("A published figure moved, or a script broke. If the new value is");
  console.error("correct, update the assertion AND the prose that quotes it.\n");
  process.exit(1);
}

console.log(`${run} of ${run} MSMB scripts ran, with their published figures intact.`);
