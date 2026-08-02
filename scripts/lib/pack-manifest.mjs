/**
 * Shared reader for `packs/<id>/pack.toml`.
 *
 * Both the verifier and the artifact builder need to agree on what a manifest
 * says. A second parser would eventually disagree with the first, and the whole
 * point of the manifest is that the published numbers cannot drift from the
 * files on disk.
 */

import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
export const packsRoot = path.join(repositoryRoot, "packs");
export const STATUSES = new Set(["solved", "partial", "blocked"]);

/**
 * Parse the manifest subset used by pack.toml: `[table]`, `[[array]]`, and
 * scalar string/bool/number/array values. Deliberately not a general TOML
 * parser — a dependency for this would be the wrong trade, and an unsupported
 * construct raises rather than being silently dropped.
 */
export function parsePackToml(source, origin) {
  const result = { pack: {}, problem: [] };
  let target = null;
  let lineNumber = 0;

  for (const raw of source.split(/\r?\n/)) {
    lineNumber += 1;
    const line = raw.replace(/(^|\s)#.*$/, "").trim();
    if (!line) continue;

    const arrayTable = line.match(/^\[\[(\w+)\]\]$/);
    if (arrayTable) {
      if (arrayTable[1] !== "problem") {
        throw new Error(`${origin}:${lineNumber}: unknown array table [[${arrayTable[1]}]]`);
      }
      target = {};
      result.problem.push(target);
      continue;
    }

    const table = line.match(/^\[(\w+)\]$/);
    if (table) {
      if (table[1] !== "pack") {
        throw new Error(`${origin}:${lineNumber}: unknown table [${table[1]}]`);
      }
      target = result.pack;
      continue;
    }

    const pair = line.match(/^([A-Za-z_][\w-]*)\s*=\s*(.+)$/);
    if (!pair || !target) throw new Error(`${origin}:${lineNumber}: cannot parse '${raw.trim()}'`);
    target[pair[1]] = parseValue(pair[2], origin, lineNumber);
  }

  return result;
}

function parseValue(text, origin, lineNumber) {
  if (text.startsWith('"')) {
    const closing = text.lastIndexOf('"');
    if (closing <= 0) throw new Error(`${origin}:${lineNumber}: unterminated string`);
    return text.slice(1, closing).replace(/\\"/g, '"');
  }
  if (text.startsWith("[")) {
    const inner = text.slice(1, text.lastIndexOf("]"));
    if (!inner.trim()) return [];
    return inner.split(",").map((item) => parseValue(item.trim(), origin, lineNumber));
  }
  if (text === "true") return true;
  if (text === "false") return false;
  if (/^-?\d+(\.\d+)?$/.test(text)) return Number(text);
  throw new Error(`${origin}:${lineNumber}: unsupported value '${text}'`);
}

export async function exists(target) {
  return stat(target).then(() => true, () => false);
}

/** Every `.bl` under a directory, recursively, in stable order. */
export async function blFiles(directory) {
  const found = [];
  for (const entry of (await readdir(directory, { withFileTypes: true }).catch(() => []))) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) found.push(...(await blFiles(absolute)));
    else if (entry.name.endsWith(".bl")) found.push(absolute);
  }
  return found.sort();
}

export async function listPackIds() {
  const entries = await readdir(packsRoot, { withFileTypes: true }).catch(() => []);
  return entries.filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();
}

export async function readPack(packId) {
  const directory = path.join(packsRoot, packId);
  const manifestPath = path.join(directory, "pack.toml");
  if (!(await exists(manifestPath))) throw new Error(`${packId}: no pack.toml`);
  const origin = path.relative(repositoryRoot, manifestPath).replaceAll("\\", "/");
  const manifest = parsePackToml(await readFile(manifestPath, "utf8"), origin);
  return { packId, directory, manifestPath, origin, manifest };
}

/** Counts published in index.json and printed by the verifier. */
export function packCounts(manifest) {
  const counts = { solved: 0, partial: 0, blocked: 0, asserted: 0, network: 0 };
  for (const problem of manifest.problem) {
    if (STATUSES.has(problem.status)) counts[problem.status] += 1;
    if (problem.asserted) counts.asserted += 1;
    if (problem.network) counts.network += 1;
  }
  return counts;
}
