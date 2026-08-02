#!/usr/bin/env node
/**
 * Build the downloadable artifacts for every example pack.
 *
 * Emits, into `dist/packs/`:
 *   <id>-<version>.json   the pack bundle — manifest plus every file inline
 *   index.json            the catalog every client reads
 *
 * Why a JSON bundle rather than a zip: the browser playground and the workbench
 * both run on WASM and would otherwise need an archive decoder in JavaScript to
 * open a pack. JSON drops straight into the existing `window.__blFiles` virtual
 * filesystem, and GitHub Pages and the release CDN both gzip it on the wire, so
 * "compressed" is handled by transport rather than by a format the client has to
 * understand. A pack is also still a plain directory in the repository, so
 * `git clone` remains a first-class way to get one.
 *
 * The catalog carries a sha256 per bundle so `bl packs add` can verify what it
 * downloaded instead of trusting the transport.
 *
 * Usage: node scripts/build-packs.mjs [--out dist/packs] [--base-url URL]
 */

import { createHash } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import {
  blFiles,
  listPackIds,
  packCounts,
  readPack,
  repositoryRoot,
} from "./lib/pack-manifest.mjs";

function flag(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

const outputRoot = path.resolve(repositoryRoot, flag("--out", path.join("dist", "packs")));
// Overridden by the release workflow with the tag's download URL. The default
// keeps a local build's catalog usable by the website without any rewriting.
const baseUrl = flag("--base-url", "/packs");

const sha256 = (text) => createHash("sha256").update(text).digest("hex");

/** Files a pack ships: its manifest, its README, and every example. */
async function bundleFiles(pack) {
  const files = {};
  const add = async (absolute) => {
    const relative = path.relative(pack.directory, absolute).replaceAll("\\", "/");
    files[relative] = await readFile(absolute, "utf8");
  };

  await add(pack.manifestPath);
  const readme = path.join(pack.directory, "README.md");
  await add(readme).catch(() => {});
  for (const absolute of await blFiles(path.join(pack.directory, "examples"))) {
    await add(absolute);
  }
  return files;
}

await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });

const packIds = await listPackIds();
if (packIds.length === 0) {
  console.error("No packs found under packs/");
  process.exit(1);
}

const catalog = [];

for (const packId of packIds) {
  const pack = await readPack(packId);
  const { manifest } = pack;
  const version = String(manifest.pack.version);
  const files = await bundleFiles(pack);
  const counts = packCounts(manifest);

  const bundle = {
    schemaVersion: 1,
    id: packId,
    version,
    // The problem list travels with the bundle so a client that has downloaded
    // a pack can render its table of contents without re-parsing pack.toml.
    pack: manifest.pack,
    problems: manifest.problem,
    files,
  };

  const bundleName = `${packId}-${version}.json`;
  const serialized = `${JSON.stringify(bundle, null, 2)}\n`;
  await writeFile(path.join(outputRoot, bundleName), serialized);

  catalog.push({
    id: packId,
    name: manifest.pack.name,
    version,
    description: manifest.pack.description,
    track: manifest.pack.track,
    listUrl: manifest.pack.list_url,
    license: manifest.pack.license,
    requires: manifest.pack.requires,
    problems: manifest.problem.length,
    counts,
    // Every problem, so a catalog reader can build a coverage table and deep
    // links without downloading the bundle first.
    index: manifest.problem.map((problem) => ({
      id: problem.id,
      title: problem.title,
      file: problem.file,
      url: problem.url,
      status: problem.status,
      asserted: Boolean(problem.asserted),
      network: Boolean(problem.network),
      blockedOn: problem.blocked_on,
    })),
    bundle: {
      file: bundleName,
      url: `${baseUrl}/${bundleName}`,
      bytes: Buffer.byteLength(serialized),
      sha256: sha256(serialized),
    },
  });

  console.log(
    `${packId}@${version}: ${Object.keys(files).length} files, ` +
      `${(Buffer.byteLength(serialized) / 1024).toFixed(1)} KiB, ` +
      `${counts.solved} solved / ${counts.partial} partial`,
  );
}

const index = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  packs: catalog,
};
await writeFile(path.join(outputRoot, "index.json"), `${JSON.stringify(index, null, 2)}\n`);

console.log(
  `\nWrote ${catalog.length} pack bundle(s) and index.json to ` +
    `${path.relative(repositoryRoot, outputRoot).replaceAll("\\", "/")}/`,
);
