# Chapter 18B: Packages, Migration, and CLI Tooling

BioLang's command-line interface covers source validation, package setup,
cross-language migration, notebook conversion, editor integration, environment
diagnostics, and machine-readable API discovery.

## Check Before You Run

`bl check` parses one or more scripts without executing them:

```bash
bl check main.bl src/qc.bl src/report.bl
```

Use it in continuous integration and before running a workflow against large
inputs. `bl run --verbose` shows execution steps, while `bl run --events`
emits versioned JSON Lines events for desktop clients and automation:

```bash
bl run --verbose main.bl
bl run --events main.bl
```

## Packages

A package is BioLang source code described by `biolang.toml`. Initialize the
current directory:

```bash
mkdir sequence-qc
cd sequence-qc
bl init --name sequence-qc
```

The command creates:

```text
sequence-qc/
  biolang.toml
  main.bl
```

Dependencies can be local paths or Git repositories:

```toml
[package]
name = "sequence-qc"
version = "0.1.0"

[dependencies]
shared = { path = "../shared" }
variants = { git = "https://github.com/example/variants.git", branch = "main" }
```

Run `bl install` to install manifest dependencies. A specific local package or
Git package can also be installed directly:

```bash
bl install ../shared
bl install variants --git https://github.com/example/variants.git --branch main
```

Packages may bundle runnable examples. They remain available after installation,
without retaining the package source repository:

```bash
bl examples variants
bl examples variants --copy variants-examples
```

The first command lists the installed package's example files. The second copies
the complete tree, including nested data or validation files, into a new or
empty working directory. During package development, the command also accepts a
local package directory in place of the installed name.

Version-only registry dependencies are recognized in the manifest but the
current CLI does not fetch them from a registry.

## Packages Versus Plugins

Packages contain BioLang modules and are installed under
`~/.biolang/packages/`. Plugins are separate processes that communicate with
BioLang through the plugin JSON protocol and are installed under
`~/.biolang/plugins/`.

```bash
bl add aligner --path ./plugins/aligner
bl plugins
bl remove aligner
```

Use a package for reusable BioLang logic. Use a plugin when an external
program, another language runtime, or process isolation is required.

## Import Python, R, and Notebooks

`bl import` converts Python, R, Jupyter, and R Markdown sources to BioLang.
Always inspect the generated code: conversion preserves intent where possible,
but library-specific calls can require manual replacement.

```bash
bl import analysis.py --validate -o analysis.bl
bl import analysis.R --validate -o analysis.bl
bl import report.ipynb --validate -o report.bl
bl import report.Rmd --validate -o report.bl
```

The source format is inferred from the extension. Use `--from` to override it,
or provide a name when reading standard input:

```bash
bl import legacy.txt --from python --validate -o legacy.bl
python generate.py | bl import - --from python --name generated.py --validate
```

`--validate` reports remaining BioLang diagnostics and exits nonzero when the
conversion still needs attention. `--json` returns the converted content and
validation result as structured JSON for editor integrations.

## Notebook Interchange

BioLang runs `.bln`, `.bl.md`, and `.ipynb` notebooks:

```bash
bl notebook analysis.bln
bl notebook analysis.bl.md
```

Convert between BioLang and Jupyter notebooks or export a self-contained HTML
report:

```bash
bl notebook study.ipynb --from-ipynb > study.bln
bl notebook study.bln --to-ipynb > study.ipynb
bl notebook study.bln --export html > study.html
```

Notebook cells share one session and execute in document order.

## Environment and Editor Integration

Run the environment doctor before relying on native tools, containers, or
network capabilities:

```bash
bl doctor
```

Editors start the language server with `bl lsp`. Tooling that needs completion,
signature, and builtin documentation can consume the same structured metadata:

```bash
bl metadata --format json > biolang-metadata.json
```

Use `bl version` to show the installed release and check for updates, then
`bl upgrade` to install the latest available release.

## Reproducible Migration Checklist

1. Convert with `bl import --validate`.
2. Replace Python or R library calls that have no direct BioLang equivalent.
3. Confirm file paths and generate or document every expected input.
4. Run `bl check` over all generated `.bl` files.
5. Compare key statistics against the original Python or R workflow.
6. Record BioLang and package versions with the analysis outputs.
