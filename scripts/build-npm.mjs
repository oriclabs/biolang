#!/usr/bin/env node
/** Build both npm WASM targets and regenerate the JavaScript builtin surface. */
import { execFileSync } from "node:child_process";
import {
  copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(readFileSync(path.join(root, "npm", "package.json"), "utf8"));
const cargo = readFileSync(path.join(root, "Cargo.toml"), "utf8");
const versionModule = readFileSync(path.join(root, "npm", "version.js"), "utf8");
const workspaceVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
if (!workspaceVersion) throw new Error("Cannot read the Rust workspace version");
if (workspaceVersion !== packageJson.version) {
  throw new Error(`npm version ${packageJson.version} does not match Rust ${workspaceVersion}`);
}
const moduleVersion = versionModule.match(/export const version\s*=\s*"([^"]+)"/)?.[1];
if (moduleVersion !== packageJson.version) {
  throw new Error(`SDK version ${moduleVersion ?? "missing"} does not match npm ${packageJson.version}`);
}

for (const [target, directory] of [["nodejs", "pkg-node"], ["web", "pkg-web"]]) {
  console.log(`Building npm ${target} runtime...`);
  execFileSync(
    "wasm-pack",
    [
      "build", "crates/bl-wasm", "--target", target,
      "--out-dir", `../../npm/${directory}`, "--release",
    ],
    { cwd: root, stdio: "inherit" },
  );
}

const npmRoot = path.join(root, "npm");
// The package root is ESM, while wasm-pack's Node loader is CommonJS. Preserve
// explicit nested package scopes in the published tarball instead of relying
// on wasm-pack's generated metadata or the consumer's package type.
writeFileSync(
  path.join(npmRoot, "pkg-node", "package.json"),
  `${JSON.stringify({ type: "commonjs" }, null, 2)}\n`,
);
writeFileSync(
  path.join(npmRoot, "pkg-web", "package.json"),
  `${JSON.stringify({ type: "module" }, null, 2)}\n`,
);
const nodeWasm = path.join(npmRoot, "pkg-node", "bl_wasm_bg.wasm");
const webWasm = path.join(npmRoot, "pkg-web", "bl_wasm_bg.wasm");
const digest = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");
if (digest(nodeWasm) !== digest(webWasm)) {
  throw new Error("Node and browser WASM payloads differ; cannot safely deduplicate them");
}

const sharedDirectory = path.join(npmRoot, "wasm");
const sharedWasm = path.join(sharedDirectory, "bl_wasm_bg.wasm");
mkdirSync(sharedDirectory, { recursive: true });
copyFileSync(nodeWasm, sharedWasm);

const rewrite = (file, from, to) => {
  const source = readFileSync(file, "utf8");
  if (!source.includes(from)) throw new Error(`Cannot find generated WASM path in ${file}`);
  writeFileSync(file, source.replace(from, to));
};
rewrite(
  path.join(npmRoot, "pkg-node", "bl_wasm.js"),
  "`${__dirname}/bl_wasm_bg.wasm`",
  "`${__dirname}/../wasm/bl_wasm_bg.wasm`",
);
rewrite(
  path.join(npmRoot, "pkg-web", "bl_wasm.js"),
  "new URL('bl_wasm_bg.wasm', import.meta.url)",
  "new URL('../wasm/bl_wasm_bg.wasm', import.meta.url)",
);
rmSync(nodeWasm);
rmSync(webWasm);
console.log(`Shared one ${readFileSync(sharedWasm).byteLength}-byte WASM payload across both loaders.`);

execFileSync(process.execPath, [path.join(root, "scripts", "generate-js-builtins.mjs")], {
  cwd: root,
  stdio: "inherit",
  env: {
    ...process.env,
    BIOLANG_WASM_MODULE: path.join(npmRoot, "pkg-web", "bl_wasm.js"),
    BIOLANG_WASM_BINARY: sharedWasm,
  },
});
