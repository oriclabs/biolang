import { constants, access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(desktopRoot, "..");
const cli = process.env.BIOLANG_CLI
  ?? path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");
const helpPath = path.join(desktopRoot, "src", "generated", "help-index.json");
const metadataPath = path.join(desktopRoot, "src", "generated", "builtin-metadata.json");
const batchSize = 80;
const runtimeSmokePaths = [
  "examples/hello.bl",
  "examples/bio.bl",
  "examples/tables.bl",
  "examples/basics/collections.bl",
  "examples/basics/control_flow.bl",
  "examples/basics/error_handling.bl",
  "examples/basics/functions.bl",
  "examples/basics/pipes.bl",
  "examples/basics/strings.bl",
  "examples/data/datetime.bl",
  "examples/data/json_ops.bl",
  "examples/data/matrices.bl",
  "examples/data/regex.bl",
  "examples/data/sparse_matrix.bl",
  "examples/data/statistics.bl",
  "examples/data/tables.bl",
  "examples/data/text_ops.bl",
  "examples/pipelines/genomic_regions.bl",
  "examples/pipelines/qc_pipeline.bl",
  "examples/pipelines/sequence_analysis.bl",
  "examples/pipelines/variant_pipeline.bl",
];

await access(cli, constants.X_OK).catch(() => {
  throw new Error(
    `BioLang CLI not found at ${cli}. Build it with 'cargo build -p bl-cli' or set BIOLANG_CLI.`,
  );
});

const help = JSON.parse(await readFile(helpPath, "utf8"));
const snippets = [];
const addSnippet = (entry, code, suffix) => {
  const normalized = code.trim();
  if (!normalized || normalized.includes("# biolang-check: skip")) return;
  snippets.push({
    label: `${entry.sourcePath ?? entry.id}${suffix}`,
    code: `${normalized}\n`,
  });
};

for (const entry of help.entries) {
  if (entry.kind === "example" && entry.code) {
    addSnippet(entry, entry.code, "");
    continue;
  }
  if (entry.kind === "builtin") {
    if (entry.example) addSnippet(entry, entry.example, ` (${entry.title} example)`);
    continue;
  }
  let block = 0;
  for (const match of entry.body.matchAll(/```(?:bio|biolang)[^\r\n]*\r?\n([\s\S]*?)```/g)) {
    block += 1;
    addSnippet(entry, match[1], ` (code block ${block})`);
  }
}

const generatedMetadata = JSON.parse(await readFile(metadataPath, "utf8"));
const { stdout: metadataOutput } = await execFileAsync(cli, ["metadata", "--format", "json"], {
  cwd: repositoryRoot,
  maxBuffer: 16 * 1024 * 1024,
});
const currentMetadata = JSON.parse(metadataOutput);
if (JSON.stringify(generatedMetadata) !== JSON.stringify(currentMetadata)) {
  throw new Error("Generated builtin metadata is stale. Run 'npm run generate:help' with the current BioLang CLI.");
}

const filter = process.env.BIOLANG_EXAMPLE_FILTER?.trim().toLowerCase();
const selectedSnippets = filter
  ? snippets.filter((snippet) => snippet.label.toLowerCase().includes(filter))
  : snippets;
if (!selectedSnippets.length) {
  throw new Error(`No inline BioLang examples matched '${process.env.BIOLANG_EXAMPLE_FILTER}'`);
}

const tempRoot = await mkdtemp(path.join(os.tmpdir(), "biolang-inline-examples-"));
const labels = new Map();
try {
  const paths = [];
  for (const [index, snippet] of selectedSnippets.entries()) {
    const filename = `${String(index + 1).padStart(5, "0")}.bl`;
    const absolute = path.join(tempRoot, filename);
    labels.set(filename, snippet.label);
    await writeFile(absolute, snippet.code, "utf8");
    paths.push(absolute);
  }

  const errors = new Map();
  for (let offset = 0; offset < paths.length; offset += batchSize) {
    const batch = paths.slice(offset, offset + batchSize);
    try {
      await execFileAsync(cli, ["check", ...batch], {
        cwd: repositoryRoot,
        maxBuffer: 16 * 1024 * 1024,
      });
    } catch (error) {
      const rawMessage = String(error.stderr || error.stdout || error.message);
      let message = rawMessage;
      for (const [filename, label] of labels) {
        message = message.replaceAll(path.join(tempRoot, filename), label);
      }
      const failedFiles = [...rawMessage.matchAll(/(\d{5}\.bl):/g)].map((match) => match[1]);
      for (const filename of new Set(failedFiles)) {
        errors.set(labels.get(filename) ?? filename, message.trim());
      }
    }
  }
  if (errors.size) {
    console.error(`${errors.size} inline BioLang examples failed syntax validation:`);
    for (const label of errors.keys()) console.error(`- ${label}`);
    if (process.env.BIOLANG_EXAMPLE_VERBOSE === "1") {
      console.error(`\n${[...new Set(errors.values())].join("\n\n")}`);
    }
    process.exitCode = 1;
  } else {
    const runtimeFailures = [];
    if (!filter) {
      for (const relativePath of runtimeSmokePaths) {
        try {
          await execFileAsync(cli, ["run", relativePath], {
            cwd: repositoryRoot,
            env: { ...process.env, BIOLANG_NO_UPDATE_CHECK: "1" },
            maxBuffer: 16 * 1024 * 1024,
          });
        } catch (error) {
          runtimeFailures.push({
            relativePath,
            message: String(error.stderr || error.stdout || error.message).trim(),
          });
        }
      }
    }
    if (runtimeFailures.length) {
      console.error(`${runtimeFailures.length} BioLang runtime smoke examples failed:`);
      for (const failure of runtimeFailures) console.error(`- ${failure.relativePath}`);
      if (process.env.BIOLANG_EXAMPLE_VERBOSE === "1") {
        console.error(`\n${runtimeFailures.map((failure) => failure.message).join("\n\n")}`);
      }
      process.exitCode = 1;
    } else {
      const counts = selectedSnippets.reduce((result, snippet) => {
        const source = snippet.label.split(" (")[0];
        const kind = source.startsWith("examples/") ? "scripts"
          : source.includes("builtin-metadata") ? "builtins"
            : "documentation";
        result[kind] = (result[kind] ?? 0) + 1;
        return result;
      }, {});
      console.log(
        `Inline BioLang examples verified: ${selectedSnippets.length} total `
        + `(${counts.scripts ?? 0} scripts, ${counts.builtins ?? 0} builtins, `
        + `${counts.documentation ?? 0} documentation blocks); `
        + `${filter ? "runtime smoke skipped for filtered check" : `${runtimeSmokePaths.length} runtime smoke scripts passed`}`,
      );
    }
  }
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}
