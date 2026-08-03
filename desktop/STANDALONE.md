# Standalone Desktop Project Boundary

BioLang Desktop should become an independent repository. The editor is a
consumer of BioLang and SOMER, not part of either runtime.

## Naming Recommendation

Keep **BioLang Desktop** as the product name and `biolang-desktop` as the
independent project/repository name for now. It immediately communicates which
language the tool supports, while SOMER remains the distinct compute-service
brand. A codename such as **ORBIT** (Omics Research BioLang Integrated Toolkit)
can be evaluated before signing and release, but should not be applied until
package names, application identifiers, domains, and trademarks are checked.

## Ownership

The Desktop repository owns:

- Tauri shell, React workbench, Monaco integration, and xterm integration.
- Workspace, editor, preview, terminal, package, API, job, and settings UI.
- Native filesystem, process, PTY, keychain, dialog, and updater adapters.
- Desktop release packaging, signing, channels, recovery, and telemetry policy.
- Compatibility declarations for supported BioLang and SOMER versions.

The BioLang repository owns:

- Language grammar, parser, evaluator, CLI, formatter, package resolution, LSP,
  runtime metadata, builtins, examples, and language documentation.

The SOMER repository owns:

- Remote identity, scheduling, execution, events, datasets, and artifacts.

## Integration Contracts

- `BIOLANG_BIN` selects an installed or bundled BioLang executable.
- `BIOLANG_SOURCE_ROOT` selects a BioLang source checkout when regenerating
  offline help during development.
- Production help metadata is consumed from the versioned `bl metadata` schema
  and retained as a generated snapshot for standalone builds. Books and examples
  still require `BIOLANG_SOURCE_ROOT` during documentation regeneration.
- Code import (Python/R/Jupyter/R Markdown → BioLang) is consumed through the
  `bl import ... --json` contract, which emits the versioned `ImportResult`
  schema on stdout. The Desktop no longer links the `bl-import` crate — it
  passes a file path, or `-` with `--name <file>` to convert piped stdin.
- `@somer/client` is the remote execution contract. The Desktop no longer
  consumes the live `../../somer` source tree — it pins an immutable, versioned
  tarball vendored at `vendor/somer-client-<version>.tgz` (integrity-locked in
  `package-lock.json`). This can graduate to a private or public registry
  release later by swapping the `file:` specifier for a version range.

## Extraction Sequence

1. Choose the product name and permanent application identifier.
2. Create the independent repository and preserve file history.
3. Publish BioLang help/metadata as a versioned build artifact.
4. Publish `@somer/client` and replace the development file dependency.
   (Done as a vendored, version-pinned tarball; a registry release can follow.)
5. Configure bundled and system BioLang runtime discovery.
6. Create independent CI, signing, updater, and release channels.

Moving the current folder before steps 1, 3, and 4 would only replace explicit
dependencies with broken relative paths. The code is therefore made
standalone-ready first and moved after the product identity is selected.
