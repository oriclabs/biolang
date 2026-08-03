#!/usr/bin/env node
/**
 * Run each pack's examples as one shared session, and refuse a top-level binding
 * that hides a builtin.
 *
 * Every other check runs an example on its own, which is not how people meet
 * them. A workbench session, a REPL, or a page that replays its blocks keeps
 * state between runs, and a top-level `let` outlives the example that wrote it.
 *
 * BINS opened with `let keys = [40, 10, ...]`. On its own that is fine. Run MAJ
 * after it in the same session and `keys(counts)` reports "List is not callable"
 * — the builtin is still there, but the name now finds a list. Nothing caught
 * it, because nothing ran two examples together.
 *
 * A binding inside a function is not a problem: it goes away with the call. Only
 * column-zero bindings are checked.
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packsDir = path.join(repositoryRoot, "packs");
const cli = path.join(
  repositoryRoot,
  "target",
  "debug",
  process.platform === "win32" ? "bl.exe" : "bl",
);

if (!fs.existsSync(cli)) {
  console.error(`No CLI at ${path.relative(repositoryRoot, cli)} — run \`cargo build -p bl-cli\` first.`);
  process.exit(2);
}

/** Builtin names, straight from the binary that will run the examples. */
function builtinNames() {
  const raw = execFileSync(cli, ["metadata", "--format", "json"], {
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
  return new Set(JSON.parse(raw).builtins.map((b) => b.name));
}

/** Problems in manifest order, with the fields this check needs. */
function problems(packId) {
  const manifest = fs.readFileSync(path.join(packsDir, packId, "pack.toml"), "utf8");
  const out = [];
  for (const block of manifest.split(/^\[\[problem\]\]$/m).slice(1)) {
    const file = block.match(/^file = "(.+)"$/m)?.[1];
    const id = block.match(/^id = "(.+)"$/m)?.[1];
    if (file) out.push({ id, file, network: /^network = true$/m.test(block) });
  }
  return out;
}

const builtins = builtinNames();
const failures = [];

for (const packId of fs.readdirSync(packsDir)) {
  if (!fs.existsSync(path.join(packsDir, packId, "pack.toml"))) continue;

  const listed = problems(packId);

  // ── 1. no top-level binding may hide a builtin
  for (const { id, file } of listed) {
    const source = fs.readFileSync(path.join(packsDir, packId, file), "utf8");
    const code = source.replace(/#.*/g, "");
    for (const match of code.matchAll(/^(?:let\s+|for\s+)([a-z_][a-z_0-9]*)\b/gm)) {
      if (builtins.has(match[1])) {
        failures.push(
          `${packId}/${id}: top-level \`${match[1]}\` hides the builtin of the same name — ` +
            `rename it, or a later example calling ${match[1]}() in the same session breaks`,
        );
      }
    }
  }

  // ── 2. the whole pack has to run as one session
  // Network examples are left out so this stays hermetic, same as `bl test`.
  const hermetic = listed.filter((p) => !p.network);
  const combined = hermetic
    .map(({ id, file }) => `# ── ${id}\n${fs.readFileSync(path.join(packsDir, packId, file), "utf8")}`)
    .join("\n");

  const scratch = path.join(repositoryRoot, "target", `session-${packId}.bl`);
  fs.writeFileSync(scratch, combined);
  try {
    execFileSync(cli, ["run", scratch], { encoding: "utf8", stdio: "pipe" });
    console.log(`${packId}: ${hermetic.length} examples run in one shared session`);
  } catch (error) {
    const output = `${error.stdout ?? ""}${error.stderr ?? ""}`.trim().split("\n").slice(0, 6).join("\n");
    failures.push(`${packId}: the pack does not run as one session\n${output}`);
  } finally {
    fs.rmSync(scratch, { force: true });
  }
}

if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):\n`);
  for (const failure of failures) console.error(`  ${failure}\n`);
  process.exit(1);
}
console.log("\nEvery pack runs as one shared session, and no example hides a builtin.");
