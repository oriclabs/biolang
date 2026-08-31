import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import { mean } from "../generated-builtins.js";
import { BioLangSession } from "../session.js";
import * as bio from "../dsl.js";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasmRoot = path.resolve(packageRoot, "..", "desktop", "public", "wasm");

test("generated JavaScript executes through the shipped WASM interpreter", async () => {
  globalThis.window = globalThis;
  globalThis.__blFiles = {};
  globalThis.__blFetch = { sync: () => "ERROR:offline integration test" };
  const wasm = await import(pathToFileURL(path.join(wasmRoot, "bl_wasm.js")).href);
  await wasm.default({ module_or_path: readFileSync(path.join(wasmRoot, "bl_wasm_bg.wasm")) });
  wasm.init();

  const session = new BioLangSession(wasm);
  assert.equal(session.runtimeVersion(), "1.5.0");
  const result = session.run(mean([1, 2, 3]));
  assert.equal(result.ok, true, result.error ?? "WASM execution failed");
  assert.equal(result.type, "Float");
  assert.equal(result.value, "2");
  const directResult = await session.mean([1, 2, 3]);
  assert.equal(directResult.ok, true, directResult.error ?? "Direct API execution failed");
  assert.equal(directResult.value, "2");

  const objectResult = session
    .table([{ Age: 20, BMI: 22 }, { Age: 15, BMI: 30 }, { Age: 40, BMI: 28 }])
    .where({ Age: { gte: 18 }, BMI: { lt: 25 } })
    .column("BMI")
    .mean()
    .run(session);
  assert.equal(objectResult.ok, true, objectResult.error ?? "Object API execution failed");
  assert.equal(objectResult.value, "22");

  const generated = session.transpileJavaScript(
    "let measurements = [12, 14, 15, 15, 16, 19, 28]\nsummary(measurements)",
  );
  assert.match(generated, /let measurements = \[12, 14, 15/);
  assert.match(generated, /await bl\.summary\(measurements\)/);
  assert.doesNotMatch(generated, /bl\.define|bl\.run/);
  assert.doesNotMatch(generated, /`let measurements/);
  const executableGenerated = generated.replace(/\nresult;\s*$/, "\nreturn result;");
  const executeJavaScript = new Function("bio", "bl", `return (async () => { ${executableGenerated} })()`);
  const generatedResult = await executeJavaScript(bio, session);
  assert.equal(generatedResult.ok, true, generatedResult.error ?? "Generated JavaScript execution failed");
  assert.equal(generatedResult.type, "Record");

  const pipeline = session.transpileJavaScript(
    "[1, 2, 3, 4] |> filter(|value| value >= 3) |> mean()",
  ).replace(/\nresult;\s*$/, "\nreturn result;");
  const pipelineResult = await new Function("bio", "bl", `return (async () => { ${pipeline} })()`)(bio, session);
  assert.equal(pipelineResult.ok, true, pipelineResult.error ?? "Generated pipeline execution failed");
  assert.equal(pipelineResult.value, "3.5");
});
