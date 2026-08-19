# Contributing to BioLang

Thank you for your interest in contributing to BioLang!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USER/biolang.git`
3. Create a branch: `git checkout -b my-feature`
4. Build: `cargo build --workspace`
5. Test: `cargo test --workspace`
6. Submit a pull request

## Development

### Prerequisites

- Rust 1.75+ (stable)
- `cargo clippy` for lints
- `cargo fmt` for formatting

### Building

```bash
cargo build --workspace          # Debug build
cargo build --workspace --release # Release build
cargo test --workspace           # Run all tests
cargo clippy --workspace         # Lint checks
```

### How lang.bio is published

Nothing in this repository deploys the site. `oriclabs/biolang-website` does,
and it is the only place that can: it checks this repository out as `_core` and
`oriclabs/biolang-workflows` as `_workflows`, then assembles one site from both.
The language reference, example packs, canonical wasm module, and browser
workbench come from here. Practical courses, notebooks, applied workflows, and
their validation live only in `biolang-workflows`. Authored static pages,
browser tests, and deployment live only in `biolang-website`.

The publisher polls on a schedule rather than being triggered, because a scoped
`GITHUB_TOKEN` cannot start a workflow in another repository. A merge here
therefore reaches lang.bio at the next poll, or through a manual website run.

### Generated Files

Two committed artifacts are built from sources elsewhere in the tree:
`desktop/src/generated` from the example packs and CLI metadata, and
`desktop/public/wasm` from the browser runtime crates. The website publisher
copies that wasm module; there is no second committed copy in this repository.
Editing a source without rerunning its generator leaves a stale artifact that
`cargo test` cannot see, so CI checks both.

The same check compares `npm/package.json` against the workspace version in
`Cargo.toml`. Nothing generates that file, but nothing synced it either, and it
sat at 1.1.0 through two tagged releases while the runtime it wraps moved on.

Enable the pre-push hook once per clone, and it catches generated metadata and
version drift before anything leaves your machine:

```bash
git config core.hooksPath .githooks
node scripts/check-generated.mjs   # or run it directly
```

The hook rewrites the stale files in place, so a failure leaves the corrected
output ready to commit. `git push --no-verify` skips it.

### Crate Structure

| Crate | What it does |
|---|---|
| `bio-core` | Pure data types (no language deps) |
| `bl-core` | AST, Value, Table, errors |
| `bl-lexer` | Tokenizer |
| `bl-parser` | Parser |
| `bl-runtime` | Interpreter + builtins |
| `bl-bio` | File I/O (FASTA, FASTQ, BED, GFF, VCF) |
| `bl-apis` | Bio API clients (NCBI, Ensembl, etc.) |
| `bl-repl` | Interactive REPL |
| `bl-lsp` | Language Server Protocol |
| `bl-cli` | CLI binary (`bl`) |

### Adding a Builtin Function

1. Add the implementation in `bl-runtime/src/builtins.rs`
2. Register it with `register(name, arity, function)` in the appropriate section
3. Add a test in `bl-runtime/tests/`
4. Document it in `oriclabs/biolang-website` under `docs/builtins/`

### Conventions

- Bio-domain types (pure data, no language deps) go in `bio-core`
- Language-runtime types (depend on `Value`, AST) go in `bl-core`
- Crates use `version.workspace = true` and `edition.workspace = true`
- `bl-*` prefix for language crates, `bio-core` is the exception

## Reporting Issues

- Use [GitHub Issues](https://github.com/oriclabs/biolang/issues)
- Include the BioLang version (`bl --version`)
- Include a minimal reproducing script if possible

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
