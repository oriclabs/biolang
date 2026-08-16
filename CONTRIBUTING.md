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
The language reference, example packs, wasm module, and browser workbench come
from here; the practical books and the HBC courses are built from
`biolang-workflows`, where they are canonical. Those books are still authored
and audited here, and `scripts/audit-biostatistics-book.ps1` reads
`books/biostatistics/book/src`, so a chapter is edited in this repository and
synced across.

`books/hbc-scrnaseq-validated` shows why the split matters: it is built from
`_workflows/courses`, so no deploy made from this repository ever contained it,
and it only reached lang.bio once the publisher took over.

`.github/workflows/verify-site.yml` runs the same build without deploying, so a
broken chapter or a missing entry point fails on the commit that caused it. The
publisher polls on a schedule rather than being triggered, because a scoped
`GITHUB_TOKEN` cannot start a workflow in another repository — so a merge here
reaches lang.bio at the next poll, not immediately.

### Generated Files

Three committed artifacts are built from sources elsewhere in the tree:
`website/docs/examples` and `desktop/src/generated` from the example packs,
and `desktop/public/wasm` as a byte-for-byte copy of `website/wasm`. Editing a
source without rerunning its generator leaves a stale artifact that `cargo
test` cannot see — CI regenerates and fails instead.

Enable the pre-push hook once per clone, and it catches all three before
anything leaves your machine:

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
4. Document it on the website under `website/docs/builtins/`

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
