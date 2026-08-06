/**
 * A fingerprint of every source the browser runtime is built from.
 *
 * The builtin-name comparison in check-wasm-fresh.mjs answers "does the module
 * expose what the source defines". That is the drift people notice, because a
 * missing builtin is a hard error in the playground. It is not the only drift.
 *
 * Rasterising large scatter plots added no builtins at all - it changed what an
 * existing one returns. The name check reported the module current while the
 * playground was still emitting a 65 MB SVG for a plot the CLI drew in 37 KB,
 * and it stayed green through a WASM rebuild that had silently failed. A check
 * that cannot fail for a whole class of change is the kind of green tick this
 * repository has been bitten by before.
 *
 * So the build records a hash of its inputs and the check recomputes it. Any
 * edit to any file the module is compiled from moves the hash, whether or not
 * it adds a name.
 *
 * Scoped to the crates bl-wasm actually depends on, so a change to bl-cli - which
 * cannot affect the browser - does not demand a five-minute rebuild.
 */

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

/** Crates in bl-wasm's dependency tree, per crates/bl-wasm/Cargo.toml. */
export const WASM_CRATES = [
  "bl-core",
  "bl-fmt",
  "bl-import",
  "bl-lexer",
  "bl-parser",
  "bl-qc",
  "bl-runtime",
  "bl-wasm",
];

function walk(directory, out) {
  let entries;
  try {
    entries = readdirSync(directory, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries.sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (entry.name.endsWith(".rs")) out.push(full);
  }
  return out;
}

/**
 * Hash the sources and the manifests.
 *
 * Not Cargo.lock, though a dependency bump does change the module without
 * touching a line of this repository's own source. The lockfile is not tracked
 * here, so cargo regenerates it in a fresh clone and it would hash differently
 * on a CI runner than on the machine that built the module - a gate that fails
 * for everyone, always, for a reason that has nothing to do with staleness.
 *
 * The manifests are hashed instead, so a changed version requirement is caught.
 * A silent upgrade within an existing requirement is not; pinning that would
 * mean committing the lockfile, which is a separate decision.
 */
export function fingerprint(repositoryRoot) {
  const files = [];
  for (const crate of WASM_CRATES) {
    const root = path.join(repositoryRoot, "crates", crate);
    walk(path.join(root, "src"), files);
    const manifest = path.join(root, "Cargo.toml");
    try {
      if (statSync(manifest).isFile()) files.push(manifest);
    } catch {
      // A crate that has been renamed or removed: the missing manifest changes
      // the file list, which changes the hash, which is the correct outcome.
    }
  }
  const digest = createHash("sha256");
  for (const file of files.sort()) {
    let contents;
    try {
      contents = readFileSync(file);
    } catch {
      continue;
    }
    // Path as well as contents, so moving code between files is a change.
    // Newlines normalised: a checkout with CRLF must not read as a rebuild.
    digest.update(path.relative(repositoryRoot, file).replaceAll("\\", "/"));
    digest.update(contents.toString("utf8").replaceAll("\r\n", "\n"));
  }
  return { hash: digest.digest("hex"), fileCount: files.length };
}
