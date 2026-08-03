#!/usr/bin/env node
/**
 * Run the docs blocks the playground marks "CLI Only" through `bl`.
 *
 * The Playwright sweep clicks everything the page will run and writes the rest
 * here — blocks that touch files, write output, or call an API without CORS.
 * Those still have to be correct, and until now nothing ran them: `bl check`
 * parses without executing, so an example that only fails at runtime passed.
 *
 * A block is compiled with `bl check` first, which is the whole test for
 * anything that needs the network or a data file that is not in the repository.
 * Blocks that need neither are then run.
 *
 * Usage:  node tests/check-doc-examples-cli.mjs [--run]
 *   --run   also execute the hermetic blocks, not just compile them
 */
import { execFile } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(websiteRoot, "..");
const cli =
  process.env.BIOLANG_CLI ??
  path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");
const inputPath = path.join(websiteRoot, "tests", "cli-only-blocks.json");
const alsoRun = process.argv.includes("--run");

if (!fs.existsSync(inputPath)) {
  console.error(
    `No ${path.relative(repositoryRoot, inputPath)} — run the Playwright sweep first:\n` +
      `  npx playwright test tests/e2e/doc-examples.spec.mjs`,
  );
  process.exit(2);
}
if (!fs.existsSync(cli)) {
  console.error(`No CLI at ${path.relative(repositoryRoot, cli)} — cargo build -p bl-cli`);
  process.exit(2);
}

/** Blocks that cannot run here, and why. Compiled but not executed. */
const NEEDS_THE_WORLD =
  /\b(read_csv|read_tsv|read_fasta|read_fastq|read_vcf|read_bed|read_sam|read_bam|read_json|read_text|read_lines|open|write_\w+|save_\w+|ncbi_\w+|ensembl_\w+|uniprot_\w+|kegg_\w+|pdb_entry|string_network|go_\w+|fetch|http_get|http_post|llm_\w+|chat)\s*\(/;

/** A shell transcript or REPL session, not a program. */
const NOT_A_PROGRAM = /^\s*(bl>|\$|#!\/)/;

const blocks = JSON.parse(fs.readFileSync(inputPath, "utf8"));
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bl-doc-cli-"));
const failures = [];
let compiled = 0;
let executed = 0;
let skipped = 0;

for (const [ordinal, block] of blocks.entries()) {
  const source = (block.source ?? "").trim();
  if (!source || NOT_A_PROGRAM.test(source)) {
    skipped += 1;
    continue;
  }

  const file = path.join(scratch, `block-${ordinal}.bl`);
  fs.writeFileSync(file, `${source}\n`);
  const where = `${block.url}#${block.index}`;

  try {
    await execFileAsync(cli, ["check", file], { timeout: 60_000 });
    compiled += 1;
  } catch (error) {
    const detail = `${error.stdout ?? ""}${error.stderr ?? ""}`.trim().split("\n")[0];
    failures.push(`${where} does not compile :: ${detail.slice(0, 200)}`);
    continue;
  }

  if (!alsoRun || NEEDS_THE_WORLD.test(source)) {
    skipped += 1;
    continue;
  }

  try {
    await execFileAsync(cli, ["run", file], { timeout: 60_000, cwd: repositoryRoot });
    executed += 1;
  } catch (error) {
    const detail = `${error.stdout ?? ""}${error.stderr ?? ""}`.trim().split("\n")[0];
    failures.push(`${where} fails at runtime :: ${detail.slice(0, 200)}`);
  }
}

fs.rmSync(scratch, { recursive: true, force: true });

console.log(
  `${blocks.length} CLI-only doc blocks: ${compiled} compiled, ${executed} run, ` +
    `${skipped} skipped as a transcript or as needing files/network`,
);
if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):\n`);
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log("Every CLI-only doc block compiles, and every hermetic one runs.");
