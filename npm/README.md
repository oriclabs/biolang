# BioLang JavaScript SDK

Write browser or Node.js code in JavaScript while BioLang's Rust/WebAssembly
runtime performs the biological and statistical computation.

```bash
npm install biolang
```

```js
import { BioLang, mean } from "biolang";

const bl = await BioLang.create();
const result = bl.run(mean([1, 2, 3]));

console.log(result.value);          // "2"
console.log(bl.runtimeVersion());   // version compiled into WASM
```

The JavaScript layer builds BioLang source and sends it through the same parser,
interpreter and builtins used by `.bl` programs. It does not implement a second
statistics or bioinformatics engine.

Existing BioLang can also be converted into inspectable structural JavaScript;
the result contains SDK calls, never a BioLang template string:

```js
const javascript = bl.transpileJavaScript(
  "let measurements = [12, 14, 15]\nsummary(measurements)"
);
// bio.program(bio.let_(...), bio.expr_(bio.callExpr(...))).run(bl)
```

## JavaScript objects and pipelines

Builtin calls return lazy `BioExpression` objects. Nothing is calculated until
the expression is passed to a session or SOMER executor.

```js
import { BioLang } from "biolang";

const bl = await BioLang.create({ cwd: "./data" });

const analysis = bl.csv("nhanes.csv")
  .where({ Age: { gte: 18 }, BMI: { lt: 40 } })
  .column("BMI")
  .mean();

console.log(analysis.toBioLang());
const result = await analysis.run(bl);
```

JavaScript cannot overload `===`, `>=`, `&&` or similar operators at runtime.
The safe portable API therefore uses `.eq()`, `.gte()` and `.and()`. BioLang
Studio can add a source transform for ordinary operator spelling without
changing this runtime API. Unsafe Proxy coercions throw rather than silently
dropping a scientific predicate.

## Complete WASM builtin coverage

Every builtin reported by the shipped WASM module has a generated JavaScript
function and TypeScript declaration:

```js
import {
  gc_content,
  histogram,
  kaplan_meier,
  reverse_complement,
  sc_sctransform
} from "biolang";
```

`npm run check:coverage` compares the two name sets exactly and fails on a
missing or stale wrapper. The current catalog is also available as
`biolang/catalog`. `session.supports(name)` performs a runtime check.

## Programs and functions

Common BioLang statements have JavaScript builders:

```js
import { function_, if_, program, ref, return_ } from "biolang";

const classify = program(
  function_(
    "classify",
    ["x"],
    if_(ref("x").gte(10), return_("high"), return_("low"))
  )
);

bl.run(classify);
```

`raw(source)` is the compatibility escape hatch for a newly added language
construct before a dedicated JavaScript builder is released.

## Live sessions

State persists between `run` calls. The session API also exposes what Studio
needs to inspect and export live values:

```js
bl.run("let values = [1, 2, 3]");
bl.variables();
bl.inspectVariable("values", { offset: 0, limit: 20 });
bl.exportVariable("values", { format: "json" });
bl.registerModule("my-package", "export let answer = 42");
bl.reset();
```

## SOMER

Install the optional shared SOMER client when native, remote or durable
execution is required:

```bash
npm install biolang @somer/client
```

```js
const somer = await bl.connectSomer({
  baseUrl: "https://somer.example.org",
  token
});

const run = await analysis.runOn(somer, {
  name: "NHANES BMI",
  resources: { cpu: 4, memoryGb: 16 },
  inputs: [{ path: "nhanes.csv", data: selectedFile }]
});

await run.events((event) => console.log(event));
const job = await run.wait();
```

Remote execution is always explicit. The SDK never uploads data because a WASM
capability is missing.

## Node and browser files

Node reads relative paths from `cwd`. Network access can be disabled:

```js
const bl = await BioLang.create({ cwd: "./data", network: false });
```

Browser applications should provide a synchronous reader backed by files that
were prepared before evaluation. Run the interpreter in a Worker so synchronous
evaluation cannot block the page:

```js
const files = new Map([["reads.fa", ">a\nACGT\n"]]);
const bl = await BioLang.create({
  fetchSync: (path) => files.get(path) ?? "ERROR:not found"
});
```

## Results

`run()` returns a typed result object rather than a JSON string:

| Field | Meaning |
|---|---|
| `ok` | Whether evaluation completed |
| `value` | Formatted final value |
| `type` | BioLang runtime type |
| `output` | Text written by `print` and `println` |
| `structured` | Structured table or plot result |
| `results` | All displayed structured values |
| `trace` | Source line associated with each display |
| `error` | Error message when execution failed |

The raw wasm-bindgen API remains available from `biolang/raw`.

MIT licensed. Documentation: [lang.bio](https://lang.bio).
