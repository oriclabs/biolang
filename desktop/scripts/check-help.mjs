import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const desktopRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const helpPath = path.join(desktopRoot, "src", "generated", "help-index.json");
const help = JSON.parse(await readFile(helpPath, "utf8"));
const kinds = ["language", "builtin", "tutorial", "example"];
const errors = [];

if (help.schemaVersion !== 1) errors.push(`Unsupported schema version: ${help.schemaVersion}`);
if (!Array.isArray(help.entries) || help.entries.length === 0) errors.push("Help index has no entries");

const ids = new Set();
for (const entry of help.entries ?? []) {
  if (!entry.id || !entry.title || !entry.body || !entry.kind) {
    errors.push(`Incomplete help entry: ${entry.id || "(missing id)"}`);
  }
  if (ids.has(entry.id)) errors.push(`Duplicate help entry id: ${entry.id}`);
  ids.add(entry.id);
}

for (const kind of kinds) {
  const actual = help.entries.filter((entry) => entry.kind === kind).length;
  if (help.counts?.[kind] !== actual) {
    errors.push(`Count mismatch for ${kind}: declared ${help.counts?.[kind]}, actual ${actual}`);
  }
}

const sourcePaths = new Set(help.entries.map((entry) => entry.sourcePath).filter(Boolean));
let internalLinks = 0;
for (const entry of help.entries) {
  for (const match of entry.body.matchAll(/\[[^\]]*]\(([^)]+)\)/g)) {
    const href = match[1].trim();
    const linkedPath = href.split("#")[0];
    if (!linkedPath.endsWith(".md") || /^(https?:|mailto:)/i.test(href)) continue;
    internalLinks += 1;
    const parts = [
      ...entry.sourcePath.split("/").slice(0, -1),
      ...linkedPath.replaceAll("\\", "/").split("/"),
    ];
    const normalized = [];
    for (const part of parts) {
      if (!part || part === ".") continue;
      if (part === "..") normalized.pop();
      else normalized.push(part);
    }
    const target = normalized.join("/");
    if (!sourcePaths.has(target)) errors.push(`Unresolved help link in ${entry.sourcePath}: ${href}`);
  }
}

if (errors.length) {
  console.error(errors.join("\n"));
  process.exitCode = 1;
} else {
  const runtimeBuiltins = help.entries.filter((entry) =>
    entry.kind === "builtin" && entry.category === "runtime").length;
  console.log(
    `Help index verified: ${help.entries.length} entries, ${internalLinks} internal links, `
    + `${runtimeBuiltins} builtins use runtime-derived signatures`,
  );
}
