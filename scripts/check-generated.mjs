#!/usr/bin/env node
/**
 * Regenerate every committed artifact and fail if that changed anything.
 *
 * The workbench help index is generated from sources elsewhere in this tree.
 *
 * Editing a source and forgetting its generated output is not a mistake the
 * ordinary gates can catch: `cargo test` and check-examples-run both pass, so
 * the tree looks green right up until CI regenerates and finds a diff. That is
 * how v1.2.0 was tagged with three stale artifacts at once, one of which would
 * have shipped a runtime two days older than the documentation describing it.
 *
 * A second check compares npm/package.json against the workspace version.
 * Nothing generates that file, but nothing synced it either, and it sat at
 * 1.1.0 through two tagged releases.
 *
 * This runs the same regenerators CI runs and reports what moved. It rewrites
 * the files in place rather than diffing into a temp directory, so a failure
 * leaves the corrected output ready to commit.
 *
 * Wired up as a pre-push hook by .githooks/pre-push; `git push --no-verify`
 * skips it. Also runnable directly:
 *
 *   node scripts/check-generated.mjs
 */

import { execFile } from "node:child_process";
import { access, constants, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cli = process.env.BIOLANG_CLI
  ?? path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");

const run = (command, args, cwd = repositoryRoot) =>
  execFileAsync(command, args, { cwd, maxBuffer: 32 * 1024 * 1024 });

/** Paths that differ from HEAD, staged or not. */
async function dirtyPaths(paths) {
  const { stdout } = await run("git", ["diff", "HEAD", "--name-only", "--", ...paths]);
  return stdout.split("\n").map((line) => line.trim()).filter(Boolean);
}

const failures = [];

// 1. Workbench help index.
//
// Built from the packs and `bl metadata`. generate-help falls back
// to the committed metadata snapshot when the CLI is missing and still reports
// success, so an unbuilt binary would quietly produce a degraded index and this
// check would blame the wrong thing. Require the binary instead.
{
  try {
    await access(cli, constants.X_OK);
  } catch {
    console.error(`Cannot verify the help index: ${cli} is not built.`);
    console.error("Run `cargo build -p bl-cli`, or set BIOLANG_CLI to a built binary.");
    process.exit(1);
  }
  await run("node", ["scripts/generate-help.mjs"], path.join(repositoryRoot, "desktop"));
  const dirty = await dirtyPaths(["desktop/src/generated"]);
  if (dirty.length) {
    failures.push({
      what: "workbench help index",
      fix: "cd desktop && npm run generate:help",
      files: dirty,
      corrected: true,
    });
  }
}

// 2. npm package version.
//
// npm/package.json wraps the WASM module built from this workspace, so a
// version it does not share is a version that describes something else.
// Nothing synced them: release.yml fires on a v* tag and builds binaries, and
// the manifest is edited by hand. It sat at 1.1.0 through v1.2.0, v1.3.0 and
// the whole of 1.4.0, so `npm install biolang` advertised a release three
// behind the runtime inside it.
//
// Compared rather than rewritten. Which version is right is a release
// decision, and guessing it here would let a typo in either file silently
// rename the other.
{
  const [cargo, manifest] = await Promise.all([
    readFile(path.join(repositoryRoot, "Cargo.toml"), "utf8"),
    readFile(path.join(repositoryRoot, "npm", "package.json"), "utf8"),
  ]);
  const workspaceVersion = cargo
    .split(/\[workspace\.package\]/)[1]
    ?.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  const npmVersion = JSON.parse(manifest).version;
  if (!workspaceVersion) {
    failures.push({
      what: "workspace version",
      fix: "check that [workspace.package] in Cargo.toml still declares a version",
      files: ["Cargo.toml"],
      corrected: false,
    });
  } else if (workspaceVersion !== npmVersion) {
    failures.push({
      what: `npm package version (${npmVersion}) does not match the workspace (${workspaceVersion})`,
      fix: `set "version": "${workspaceVersion}" in npm/package.json, or correct Cargo.toml`,
      files: ["npm/package.json"],
      corrected: false,
    });
  }
}

if (failures.length) {
  console.error("\nGenerated files are out of date with their sources:\n");
  for (const failure of failures) {
    console.error(`  ${failure.what}`);
    for (const file of failure.files) console.error(`    ${file}`);
    console.error(`    fix: ${failure.fix}\n`);
  }
  // Named rather than counted. This used to read "the first two", which was
  // wrong whenever the corrected checks were not the first two to fail.
  const corrected = failures.filter((failure) => failure.corrected);
  if (corrected.length) {
    const names = corrected.map((failure) => failure.what).join(" and ");
    console.error(`Already corrected in your working tree - commit them: ${names}.`);
  }
  console.error("To push anyway: git push --no-verify\n");
  process.exit(1);
}

console.log(
  "Generated files are current: workbench help index and npm version.",
);
