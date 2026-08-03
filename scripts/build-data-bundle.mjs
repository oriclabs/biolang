#!/usr/bin/env node
/**
 * Bundle the shared sample data into one file the workbench can install.
 *
 * Examples in the documentation read `data/counts.csv`, `data/sample.fastq` and
 * three dozen others. Those files sit under website/books/data and are served to
 * the playground through its fetch bridge, but a workbench workspace never had
 * them: open a tutorial there and every read failed. Packs did not have this
 * problem because a pack example is self-contained, which is also why they were
 * the only things with an "Open in the workbench" link.
 *
 * The whole set is 121 KB, so it is shipped as one JSON blob rather than
 * fetched a file at a time.
 *
 * Usage:  node scripts/build-data-bundle.mjs [--out website/packs]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(repositoryRoot, "website", "books", "data");

const outIndex = process.argv.indexOf("--out");
const outDir = path.resolve(
  repositoryRoot,
  outIndex >= 0 ? process.argv[outIndex + 1] : path.join("website", "packs"),
);

if (!fs.existsSync(source)) {
  console.error(`No data directory at ${path.relative(repositoryRoot, source)}`);
  process.exit(2);
}

/** Text formats only. A workspace file is a string; binary would need base64. */
const TEXT = /\.(csv|tsv|txt|fa|fasta|fastq|fq|vcf|bed|gff3?|gtf|json|nwk|newick|sam|md)$/i;

const files = {};
let skipped = 0;
let bytes = 0;

for (const entry of fs.readdirSync(source, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
  if (!entry.isFile()) continue;
  if (!TEXT.test(entry.name)) {
    skipped += 1;
    continue;
  }
  const content = fs.readFileSync(path.join(source, entry.name), "utf8");
  files[`data/${entry.name}`] = content;
  bytes += Buffer.byteLength(content, "utf8");
}

fs.mkdirSync(outDir, { recursive: true });
const target = path.join(outDir, "data-bundle.json");
fs.writeFileSync(
  target,
  `${JSON.stringify({ generated: true, files }, null, 2)}\n`,
  "utf8",
);

console.log(
  `Wrote ${Object.keys(files).length} data files (${(bytes / 1024).toFixed(1)} KiB` +
    `${skipped ? `, ${skipped} skipped as binary` : ""}) to ` +
    `${path.relative(repositoryRoot, target)}`,
);
