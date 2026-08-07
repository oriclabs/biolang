# biolang

BioLang's sequence, statistics and alignment core for Node and the browser,
compiled to WebAssembly. No Rust toolchain, no native build step.

```bash
npm install biolang
```

```js
import { BioLang } from "biolang";

const bl = await BioLang.create();

const result = bl.run(`
  let seqs = read_fasta("reads.fa")
  seqs |> filter(|r| gc_content(r.seq) > 0.5) |> count()
`);

console.log(result.value);   // "128"
console.log(result.type);    // "Int"
console.log(result.output);  // anything print/println wrote
```

## What you get

`run()` returns an object rather than a JSON string:

| field | |
|---|---|
| `ok` | whether evaluation completed |
| `value` | the final expression, formatted |
| `type` | its runtime type — `Int`, `DNA`, `Table`, … |
| `output` | everything `print` and `println` wrote |
| `structured` | a table or chart as JSON, when the value is one |
| `trace` | which line produced what |
| `error` | message when `ok` is false |

State persists between `run` calls on the same instance, so a variable defined
in one is visible in the next. `reset()` gives a fresh interpreter.

Other methods: `builtins()`, `variables()`, `format(source)`, `tokenize(source)`,
`import(source, "python" | "r" | "jupyter" | "rmd")`.

## What is and is not in this build

**791 of BioLang's 1018 builtins.** The WebAssembly build compiles the runtime
without its native feature set, so 227 are absent. This is the honest table
rather than a feature list:

| Area | In this build |
|---|---|
| Sequences — FASTA/FASTQ/VCF/BED/GFF parsing, GC, k-mers, translation, reverse complement | **35 of 45** |
| Alignment, edit distance, motifs, assembly graphs, phylogenetics | yes |
| Statistics, maths, tables, lists, strings | yes |
| Single-cell | **31 of 33** |
| Filesystem builtins (`glob`, paths, directories) | **none** — 0 of 29 |
| Transfer (FTP, SSH, S3) | **none** — 0 of 15 |
| Containers / BioContainers | **none** — 0 of 10 |
| API clients (NCBI, Ensembl, UniProt, KEGG, …) | **2 of 8** |
| LLM (`chat`, `chat_code`) | **none** |
| Enrichment (`enrich`, `gsea`) | **none** |
| **Compressed input — `.gz`, `.zst`, `.lz4`** | **none** |
| SQLite, Parquet, PDF | **none** |

Two of those are worth saying twice. **Gzipped files do not work** — `flate2`
is not compiled in, so `read_fasta("reads.fa.gz")` fails. And the **API clients
are mostly absent**, so `ncbi_gene()` and friends are not available here even
though the language has them.

If you need those, use the CLI: same language, same code, all 1018 builtins.

```bash
curl -fsSL https://lang.bio/install.sh | sh     # Linux, macOS
iwr -useb https://lang.bio/install.ps1 | iex    # Windows
```

## Reading files

The interpreter calls a synchronous hook when it reads a file or URL, because
it cannot await mid-evaluation. This package installs one for you:

* **Node** — local paths read from disk, `http(s)` fetched through `curl`.
  Set `cwd` to change the base for relative paths, and `network: false` to
  refuse remote reads.
* **Browsers and bundlers** — the fallback is a synchronous `XMLHttpRequest`,
  which is deprecated on the main thread and unavailable in workers. Pass your
  own `fetchSync` if you have an in-memory workspace or a cache.

```js
// Node: confine it to one directory, no network
const bl = await BioLang.create({ cwd: "./data", network: false });

// Browser: serve files from memory
const files = { "reads.fa": ">a\nACGT\n" };
const bl = await BioLang.create({ fetchSync: (url) => files[url] ?? "ERROR:not found" });
```

## Size

The WebAssembly module is about 6 MB. It loads once per process.

## Raw module

`biolang/raw` exports the wasm-bindgen module directly, for anything the
wrapper does not cover. `evaluate()` there returns a JSON string.

```js
import * as raw from "biolang/raw";
```

## Links

* [lang.bio](https://lang.bio) — documentation, and a browser workbench that
  runs this same module
* [Embedding guide](https://lang.bio/docs/tools/embedding.html)
* [GitHub](https://github.com/oriclabs/biolang) — issues and source

MIT.
