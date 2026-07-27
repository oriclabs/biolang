import { constants, access, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configuredSourceRoot = process.env.BIOLANG_SOURCE_ROOT;
const siblingSourceRoot = path.resolve(desktopRoot, "..", "..", "biolang");
const repositoryRoot = configuredSourceRoot
  ? path.resolve(configuredSourceRoot)
  : desktopRoot.endsWith(`${path.sep}biolang${path.sep}desktop`)
    ? path.resolve(desktopRoot, "..")
    : siblingSourceRoot;
const outputPath = path.join(desktopRoot, "src", "generated", "help-index.json");
const metadataPath = path.join(desktopRoot, "src", "generated", "builtin-metadata.json");
const execFileAsync = promisify(execFile);

const read = (relativePath) => readFile(path.join(repositoryRoot, relativePath), "utf8");
const cleanInline = (value) => value
  .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
  .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
  .replace(/[`*_>#]/g, "")
  .replace(/\s+/g, " ")
  .trim();

function firstSummary(markdown) {
  const withoutCode = markdown.replace(/```[\s\S]*?```/g, "");
  const paragraphs = withoutCode.split(/\r?\n\s*\r?\n/);
  for (const paragraph of paragraphs) {
    const cleaned = cleanInline(paragraph);
    if (cleaned && !/^[-|]/.test(cleaned) && !/^Summary$/i.test(cleaned)) return cleaned.slice(0, 240);
  }
  return "BioLang documentation";
}

function markdownTitle(markdown, fallback) {
  const match = markdown.match(/^#\s+(.+)$/m);
  return match ? cleanInline(match[1]) : fallback;
}

async function summaryEntries({ summaryPath, sourceRoot, kind, collection }) {
  const summary = await read(summaryPath);
  const entries = [];
  let category = collection;
  for (const line of summary.split(/\r?\n/)) {
    const heading = line.match(/^#\s+(.+)$/);
    if (heading) {
      category = cleanInline(heading[1]);
      continue;
    }
    const link = line.match(/^(?:-\s+)?\[([^\]]+)\]\(([^)]+\.md)\)/);
    if (!link) continue;
    const relativeSource = path.posix.join(sourceRoot.replaceAll("\\", "/"), link[2].replace(/^\.\//, ""));
    const body = await read(relativeSource);
    const title = markdownTitle(body, cleanInline(link[1]));
    entries.push({
      id: `${kind}:${relativeSource}`,
      kind,
      title,
      category,
      collection,
      summary: firstSummary(body),
      body,
      sourcePath: relativeSource,
      keywords: `${title} ${category} ${collection}`.toLowerCase(),
    });
  }
  return entries;
}

function decodeRustString(value) {
  try {
    return JSON.parse(`"${value}"`);
  } catch {
    return value.replaceAll('\\"', '"').replaceAll("\\\\", "\\");
  }
}

function parseRustTuples(block) {
  const tuples = [];
  const pattern = /^\s*\("((?:\\.|[^"\\])*)",\s*"((?:\\.|[^"\\])*)",\s*"((?:\\.|[^"\\])*)"\),/gm;
  for (const match of block.matchAll(pattern)) {
    tuples.push(match.slice(1).map(decodeRustString));
  }
  return tuples;
}

function rustConstant(source, name) {
  const start = source.indexOf(`const ${name}`);
  if (start < 0) throw new Error(`${name} was not found in bl-repl`);
  const end = source.indexOf("\n];", start);
  if (end < 0) throw new Error(`${name} is not terminated`);
  return source.slice(start, end + 3);
}

async function rustFiles(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await rustFiles(absolute));
    else if (entry.name.endsWith(".rs")) result.push(absolute);
  }
  return result;
}

async function builtinEntries() {
  const descriptions = new Map();
  const appendix = await read("books/language/src/appendix-builtins.md");
  for (const match of appendix.matchAll(/^\|\s*`([^`]+)`\s*\|\s*([^|]+)\|/gm)) {
    const signature = match[1].trim();
    descriptions.set(signature.split("(")[0], cleanInline(match[2]));
  }

  const metadata = await loadBuiltinMetadata();
  return metadata.builtins.map((builtin) => {
    const { name, signature, category, example } = builtin;
    const description = descriptions.get(name) ?? builtin.summary;
    const body = [
      `# ${name}`,
      "",
      description,
      "",
      "## Signature",
      "",
      "```biolang",
      signature,
      "```",
      ...(example ? ["", "## Example", "", "```biolang", example, "```"] : []),
      ...(builtin.returnType ? ["", `**Returns:** \`${builtin.returnType}\``] : []),
      "",
      `**Arity:** ${builtin.arity.kind}, minimum ${builtin.arity.minimum}`
        + (builtin.arity.maximum == null ? "" : `, maximum ${builtin.arity.maximum}`),
    ].join("\n");
    return {
      id: `builtin:${name}`,
      kind: "builtin",
      title: name,
      category,
      collection: "Runtime builtins",
      summary: description,
      body,
      signature,
      example,
      returnType: builtin.returnType,
      sourcePath: "desktop/src/generated/builtin-metadata.json",
      keywords: `${name} ${signature} ${category} ${description}`.toLowerCase(),
    };
  });
}

async function loadBuiltinMetadata() {
  const executable = process.env.BIOLANG_CLI
    ?? path.join(repositoryRoot, "target", "debug", process.platform === "win32" ? "bl.exe" : "bl");
  try {
    await access(executable, constants.X_OK);
    const { stdout } = await execFileAsync(executable, ["metadata", "--format", "json"], {
      cwd: repositoryRoot,
      maxBuffer: 16 * 1024 * 1024,
    });
    const metadata = JSON.parse(stdout);
    if (metadata.schemaVersion !== 1 || !Array.isArray(metadata.builtins)) {
      throw new Error("unsupported metadata schema");
    }
    const serialized = `${JSON.stringify(metadata, null, 2)}\n`;
    let current = "";
    try {
      current = await readFile(metadataPath, "utf8");
    } catch {
      // First generation creates the metadata snapshot.
    }
    if (current !== serialized) await writeFile(metadataPath, serialized, "utf8");
    return metadata;
  } catch (error) {
    try {
      const cached = JSON.parse(await readFile(metadataPath, "utf8"));
      if (cached.schemaVersion === 1 && Array.isArray(cached.builtins)) return cached;
    } catch {
      // Report the authoritative CLI failure below when no snapshot exists.
    }
    throw new Error(`Cannot load BioLang builtin metadata: ${error.message}`);
  }
}

async function exampleFiles(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await exampleFiles(absolute));
    else if (entry.name.endsWith(".bl")) result.push(absolute);
  }
  return result;
}

async function exampleEntries() {
  const root = path.join(repositoryRoot, "examples");
  const entries = [];
  for (const absolute of await exampleFiles(root)) {
    const code = await readFile(absolute, "utf8");
    const relative = path.relative(repositoryRoot, absolute).replaceAll("\\", "/");
    const withinExamples = path.relative(root, absolute).replaceAll("\\", "/");
    const category = withinExamples.includes("/") ? withinExamples.split("/")[0] : "featured";
    const firstComment = code.split(/\r?\n/).find((line) => /^#\s+\S/.test(line));
    const fallback = path.basename(absolute, ".bl").replaceAll(/[-_]/g, " ");
    const title = cleanInline(firstComment?.replace(/^#\s*/, "") ?? fallback);
    const body = `# ${title}\n\n**Source:** \`${relative}\`\n\n\`\`\`biolang\n${code.trim()}\n\`\`\``;
    entries.push({
      id: `example:${relative}`,
      kind: "example",
      title,
      category,
      collection: "Repository examples",
      summary: firstComment ? cleanInline(firstComment.replace(/^#\s*/, "")) : `BioLang ${category} example.`,
      body,
      code,
      sourcePath: relative,
      keywords: `${title} ${category} ${relative} ${code.slice(0, 2_000)}`.toLowerCase(),
    });
  }
  return entries.sort((a, b) => a.sourcePath.localeCompare(b.sourcePath));
}

const language = await summaryEntries({
  summaryPath: "books/language/src/SUMMARY.md",
  sourceRoot: "books/language/src",
  kind: "language",
  collection: "BioLang Language Guide",
});
const practical = await summaryEntries({
  summaryPath: "books/practical-bioinformatics/book/src/SUMMARY.md",
  sourceRoot: "books/practical-bioinformatics/book/src",
  kind: "tutorial",
  collection: "Practical Bioinformatics in 30 Days",
});
const biostatistics = await summaryEntries({
  summaryPath: "books/biostatistics/book/src/SUMMARY.md",
  sourceRoot: "books/biostatistics/book/src",
  kind: "tutorial",
  collection: "Biostatistics in 30 Days",
});
const builtins = await builtinEntries();
const examples = await exampleEntries();
const entries = [...language, ...builtins, ...practical, ...biostatistics, ...examples];

await mkdir(path.dirname(outputPath), { recursive: true });
const output = `${JSON.stringify({
  schemaVersion: 1,
  counts: {
    language: language.length,
    builtin: builtins.length,
    tutorial: practical.length + biostatistics.length,
    example: examples.length,
  },
  entries,
}, null, 2)}\n`;
let current = "";
try {
  current = await readFile(outputPath, "utf8");
} catch {
  // The first generation creates the output file.
}
if (current !== output) {
  await writeFile(outputPath, output, "utf8");
  console.log(`Generated ${entries.length} help entries at ${path.relative(desktopRoot, outputPath)}`);
} else {
  console.log(`Help index is current (${entries.length} entries)`);
}
