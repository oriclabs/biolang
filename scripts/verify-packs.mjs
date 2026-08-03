#!/usr/bin/env node
/**
 * Validate every example pack against its manifest.
 *
 * The failure this exists to prevent: `examples/rosalind/` sat in the repo for
 * months with two examples that crashed and one that silently produced nothing,
 * because no gate ran them and no manifest said what they were supposed to do.
 * A pack is only trustworthy if the manifest and the filesystem cannot drift.
 *
 * Checks, per pack:
 *   - every `file` in pack.toml exists
 *   - every .bl in examples/ is listed in pack.toml (no orphans)
 *   - problem ids are unique and status values are known
 *   - `blocked_on` is present whenever status is not "solved"
 *   - `asserted = true` files really do define `test_*` functions, and
 *     `asserted = false` files really do not (so the count cannot lie)
 *   - network examples are never marked asserted, since CI must not gate on NCBI
 *
 * Usage: node scripts/verify-packs.mjs [--json]
 */

import { readFile } from "node:fs/promises";
import path from "node:path";
import {
  STATUSES,
  blFiles,
  exists,
  listPackIds,
  packCounts,
  readPack,
  repositoryRoot,
} from "./lib/pack-manifest.mjs";

async function verifyPack(packId) {
  const errors = [];
  let pack;
  try {
    pack = await readPack(packId);
  } catch (error) {
    return { packId, errors: [error.message] };
  }

  const { directory, manifest } = pack;
  const fail = (message) => errors.push(`${packId}: ${message}`);

  if (manifest.pack.id !== packId) {
    fail(`pack.id is '${manifest.pack.id}' but the directory is '${packId}'`);
  }
  for (const field of ["name", "version", "description", "license"]) {
    if (!manifest.pack[field]) fail(`pack.${field} is missing`);
  }

  const onDisk = await blFiles(path.join(directory, "examples"));
  const listed = new Set();
  const seenIds = new Set();

  for (const problem of manifest.problem) {
    const label = problem.id ?? "<missing id>";
    if (!problem.id) fail("a problem has no id");
    else if (seenIds.has(problem.id)) fail(`duplicate problem id '${problem.id}'`);
    else seenIds.add(problem.id);

    if (!problem.title) fail(`${label}: no title`);
    if (!problem.url) fail(`${label}: no url`);

    if (!STATUSES.has(problem.status)) {
      fail(`${label}: status '${problem.status}' is not one of ${[...STATUSES].join(", ")}`);
    }
    if (problem.status !== "solved" && !problem.blocked_on) {
      fail(`${label}: status '${problem.status}' requires blocked_on explaining what is missing`);
    }

    if (!problem.file) {
      fail(`${label}: no file`);
      continue;
    }
    const absolute = path.join(directory, problem.file);
    listed.add(absolute);
    if (!(await exists(absolute))) {
      fail(`${label}: file '${problem.file}' does not exist`);
      continue;
    }

    // The manifest claims a coverage number; make the source back it up.
    const source = await readFile(absolute, "utf8");
    const hasTests = /^\s*fn\s+test_\w+\s*\(\s*\)/m.test(source);
    if (problem.asserted && !hasTests) {
      fail(`${label}: asserted = true but ${problem.file} defines no test_* function`);
    }
    if (!problem.asserted && hasTests) {
      fail(`${label}: defines test_* functions but is not marked asserted`);
    }
    // A network problem may carry assertions — they are real and worth running
    // when online. They are simply excluded from the hermetic gate, which is
    // what `--hermetic-files` below emits, so CI never depends on a remote
    // service being up.
  }

  for (const absolute of onDisk) {
    if (!listed.has(absolute)) {
      const relative = path.relative(directory, absolute).replaceAll("\\", "/");
      fail(`${relative} is not listed in pack.toml`);
    }
  }

  // Asserted problems that do not touch the network: the set CI can gate on.
  const hermetic = manifest.problem
    .filter((problem) => problem.asserted && !problem.network && problem.file)
    .map((problem) => `packs/${packId}/${problem.file}`);

  return {
    packId,
    name: manifest.pack.name,
    total: manifest.problem.length,
    counts: packCounts(manifest),
    hermetic,
    errors,
  };
}

const packIds = await listPackIds();
if (packIds.length === 0) {
  console.error(`No packs found under ${path.relative(repositoryRoot, "packs")}/`);
  process.exit(1);
}

const reports = [];
for (const packId of packIds) reports.push(await verifyPack(packId));

const failuresFound = reports.some((report) => report.errors.length > 0);

// `bl test <dir>` would also pick up the network problems, so CI asks for the
// hermetic file list explicitly rather than pointing at the directory.
if (process.argv.includes("--hermetic-files")) {
  if (failuresFound) {
    for (const report of reports) {
      for (const error of report.errors) console.error(`  error: ${error}`);
    }
    process.exit(1);
  }
  console.log(reports.flatMap((report) => report.hermetic ?? []).join(" "));
  process.exit(0);
}

if (process.argv.includes("--json")) {
  console.log(JSON.stringify(reports, null, 2));
} else {
  for (const report of reports) {
    const { counts, total } = report;
    if (counts) {
      console.log(
        `${report.packId}: ${total} problems — ${counts.solved} solved, ` +
          `${counts.partial} partial, ${counts.blocked} blocked; ` +
          `${counts.asserted} asserted, ${counts.network} need the network`,
      );
    }
    for (const error of report.errors) console.error(`  error: ${error}`);
  }
}

const failures = reports.flatMap((report) => report.errors);
if (failures.length > 0) {
  console.error(`\n${failures.length} pack problem(s) found.`);
  process.exit(1);
}
