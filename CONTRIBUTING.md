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
