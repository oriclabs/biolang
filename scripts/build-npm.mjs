#!/usr/bin/env node
/** Build both npm WASM targets and regenerate the JavaScript builtin surface. */
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(readFileSync(path.join(root, "npm", "package.json"), "utf8"));
const cargo = readFileSync(path.join(root, "Cargo.toml"), "utf8");
const browser = readFileSync(path.join(root, "npm", "browser.js"), "utf8");
const workspaceVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
if (!workspaceVersion) throw new Error("Cannot read the Rust workspace version");
if (workspaceVersion !== packageJson.version) {
  throw new Error(`npm version ${packageJson.version} does not match Rust ${workspaceVersion}`);
}
const browserVersion = browser.match(/export const version\s*=\s*"([^"]+)"/)?.[1];
if (browserVersion !== packageJson.version) {
  throw new Error(`browser SDK version ${browserVersion ?? "missing"} does not match npm ${packageJson.version}`);
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

execFileSync(process.execPath, [path.join(root, "scripts", "generate-js-builtins.mjs")], {
  cwd: root,
  stdio: "inherit",
});
