import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import { mean } from "../generated-builtins.js";
import { BioLangSession } from "../session.js";
import * as bio from "../dsl.js";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasmRoot = path.resolve(packageRoot, "pkg-web");

test("generated JavaScript executes through the shipped WASM interpreter", async () => {
  globalThis.window = globalThis;
  globalThis.__blFiles = {};
  globalThis.__blFetch = { sync: () => "ERROR:offline integration test" };
  const wasm = await import(pathToFileURL(path.join(wasmRoot, "bl_wasm.js")).href);
  await wasm.default({
    module_or_path: readFileSync(path.join(packageRoot, "wasm", "bl_wasm_bg.wasm")),
  });
  wasm.init();

  // Keep the Rust boundary defensive even when a caller bypasses the public
  // JavaScript encoder and invokes the generated WASM class directly.
  const rawSession = new wasm.WasmSession();
  for (const value of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    assert.throws(
      () => rawSession.set_value("nonfinite", value),
      /non-finite JavaScript numbers are not supported/,
    );
  }
  assert.throws(
    () => rawSession.set_value("matrix", {
      __biolangType: "matrix",
      nrow: 1,
      ncol: 2,
      data: new Float64Array([1, Number.NaN]),
      rowNames: null,
      columnNames: null,
    }),
    /matrix data contains a non-finite JavaScript number/,
  );
  rawSession.free();

  const session = new BioLangSession(wasm);
  assert.equal(session.runtimeVersion(), "1.5.0");
  const result = session.run(mean([1, 2, 3]));
  assert.equal(result.ok, true, result.error ?? "WASM execution failed");
  assert.equal(result.type, "Float");
  assert.equal(result.value, "2");
  const directResult = await session.mean([1, 2, 3]);
  assert.equal(directResult, 2);

  const objectResult = session
    .tableExpression([{ Age: 20, BMI: 22 }, { Age: 15, BMI: 30 }, { Age: 40, BMI: 28 }])
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
  assert.match(generated, /bl\.summary\(measurements\)/);
  assert.doesNotMatch(generated, /bl\.define|bl\.run/);
  assert.doesNotMatch(generated, /`let measurements/);
  const executableGenerated = generated.replace(/\nresult;\s*$/, "\nreturn result;");
  const executeJavaScript = new Function("bio", "bl", `return (async () => { ${executableGenerated} })()`);
  const generatedResult = await executeJavaScript(bio, session);
  assert.equal(generatedResult.mean, 17);
  assert.equal(generatedResult.median, 15);

  const directTable = session.table([{ name: "Alice", score: 92 }, { name: "Bob", score: 78 }]);
  assert.deepEqual([...directTable].map(row => row.name), ["Alice", "Bob"]);
  assert.deepEqual(directTable.name, ["Alice", "Bob"]);
  assert.deepEqual(session.addValues(["a"], ["b"]), ["a", "b"]);
  assert.equal(session.equalValues([1, { value: 2 }], [1, { value: 2 }]), true);
  assert.equal(session.indexValue(["a", "b"], -1), "b");
  assert.equal(session.indexValue("ATGC", -1), "C");

  const pipeline = session.transpileJavaScript(
    "[1, 2, 3, 4] |> filter(|value| value >= 3) |> mean()",
  ).replace(/\nresult;\s*$/, "\nreturn result;");
  const pipelineResult = await new Function("bio", "bl", `return (async () => { ${pipeline} })()`)(bio, session);
  assert.equal(pipelineResult, 3.5, "Generated pipeline execution failed");

  const builtinReference = session.transpileJavaScript(
    'filter([dna"ATGC", rna"AUGC", dna"NNNN"], is_dna)',
  ).replace(/\nresult;\s*$/, "\nreturn result;");
  assert.match(builtinReference, /bl\.filter\(/);
  assert.match(builtinReference, /, bl\.isDna\)/);
  const validDna = await new Function(
    "bio", "bl", `return (async () => { ${builtinReference} })()`,
  )(bio, session);
  assert.deepEqual(validDna.map((sequence) => sequence.data), ["ATGC", "NNNN"]);

  const builtinAsValue = session.transpileJavaScript("type(len)")
    .replace(/\nresult;\s*$/, "\nreturn result;");
  assert.match(builtinAsValue, /bl\.type\(bl\.len\)/);
  assert.equal(
    await new Function("bio", "bl", `return (async () => { ${builtinAsValue} })()`)(bio, session),
    "Function",
  );

  const inheritedName = session.transpileJavaScript('to_string(protein"MKT")')
    .replace(/\nresult;\s*$/, "\nreturn result;");
  assert.match(inheritedName, /bl\.toString\(bl\.protein/);
  assert.equal(
    await new Function("bio", "bl", `return (async () => { ${inheritedName} })()`)(bio, session),
    "MKT",
  );

  const languageFeatures = session.transpileJavaScript(`
let mu = 12.3456
let base = "A"
{
  formatted: f"mean={mu:.2f}",
  label: match base { "A" => "adenine", _ => "other" },
  recovered: try { assert false, "boom" } catch err { err },
  legacy: map([1, 2], fn(value) -> value + 1)
}
  `).replace(/\nresult;\s*$/, "\nreturn result;");
  const featureResult = await new Function(
    "bio",
    "bl",
    `return (async () => { ${languageFeatures} })()`,
  )(bio, session);
  assert.equal(featureResult.formatted, "mean=12.35");
  assert.equal(featureResult.label, "adenine");
  assert.match(featureResult.recovered, /boom/);
  assert.deepEqual(featureResult.legacy, [2, 3]);
});
