import assert from "node:assert/strict";
import test from "node:test";

import {
  call,
  for_,
  function_,
  if_,
  lambda,
  let_,
  literal,
  program,
  ref,
  return_,
} from "../dsl.js";
import { mean, read_csv, WASM_BUILTIN_NAMES } from "../generated-builtins.js";

test("direct builtins generate BioLang without executing JavaScript algorithms", () => {
  assert.equal(mean([1, 2, 3]).toBioLang(), "mean([1, 2, 3])");
  assert.equal(read_csv("data.csv").toBioLang(), 'read_csv("data.csv")');
  assert.equal(WASM_BUILTIN_NAMES.length, new Set(WASM_BUILTIN_NAMES).size);
  assert.ok(WASM_BUILTIN_NAMES.length > 800);
});

test("object expressions build readable pipelines", () => {
  const adults = read_csv("nhanes.csv").filter(
    lambda("row", (row) => row.Age.gte(18).and(row.BMI.lt(40))),
  );
  const analysis = adults.column("BMI").mean();
  assert.match(analysis.toBioLang(), /read_csv\("nhanes\.csv"\)/);
  assert.match(analysis.toBioLang(), /row\)\.Age >= 18/);
  assert.match(analysis.toBioLang(), /and/);
  assert.match(analysis.toBioLang(), /^mean\(col\(/);
});

test("literal encoding cannot inject BioLang source", () => {
  const hostile = '") |> write_csv("stolen.csv") #';
  assert.equal(
    call("print", hostile).toBioLang(),
    'print("\\\") |> write_csv(\\\"stolen.csv\\\") #")',
  );
  assert.equal(literal({ "not an identifier": hostile }).toBioLang(),
    '{"not an identifier": "\\\") |> write_csv(\\\"stolen.csv\\\") #"}');
});

test("lambda safety rejects JavaScript operators that silently discard expressions", () => {
  assert.throws(
    () => lambda("row", (row) => row.Age && row.BMI),
    /constructed but discarded/,
  );
  assert.throws(
    () => lambda("row", (row) => row.Age === 18),
    /must return a BioLang expression/,
  );
  assert.throws(
    () => lambda("row", (row) => row.Age >= 18),
    /cannot use JavaScript operators directly/,
  );
});

test("statement helpers cover ordinary program structure", () => {
  const source = program(
    function_("classify", ["x"], if_(ref("x").gte(10), return_("high"), return_("low"))),
    let_("labels", []),
    for_("value", [5, 15], call("push", ref("labels"), call("classify", ref("value")))),
    ref("labels"),
  ).toBioLang();
  assert.match(source, /^fn classify\(x\)/);
  assert.match(source, /if \(x >= 10\)/);
  assert.match(source, /for value in \[5, 15\]/);
});
