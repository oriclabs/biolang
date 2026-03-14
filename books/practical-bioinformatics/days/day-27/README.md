# Day 27: Building Tools and Plugins

| | |
|---|---|
| **Difficulty** | Intermediate--Advanced |
| **Biology knowledge** | Intermediate (sequence analysis, QC metrics, file formats) |
| **Coding knowledge** | Intermediate--Advanced (functions, modules, records, pipes, error handling) |
| **Time** | ~3--4 hours |
| **Prerequisites** | Days 1--26 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (simulated sequences, QC data) |

## Quick Start

```bash
cd days/day-27
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Extracting reusable functions into BioLang modules
- Using `import` and `import ... as` to organize code across files
- Building a sequence utilities library with validation and error handling
- Building a QC module for quality control workflows
- Testing modules with `assert()`
- Understanding the plugin architecture (subprocess JSON protocol)
- Building a Python plugin that extends BioLang
- Packaging and sharing tools

## Files

| File | Description |
|------|-------------|
| `init.bl` | Generates example sequences, sets up lib/ directory with modules |
| `scripts/analysis.bl` | Full pipeline using custom modules (BioLang) |
| `scripts/analysis.py` | Python equivalent using packages/modules |
| `scripts/analysis.R` | R equivalent using custom functions in sourced files |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected output structure |

## Key Concepts

### Module System
- `import "path.bl"` loads a file and makes its functions available
- `import "path.bl" as name` creates a namespace to avoid collisions
- Modules are cached by canonical path; circular imports are rejected

### Plugin System
- Plugins use a subprocess JSON protocol (stdin/stdout)
- Manifest: `plugin.json` in `~/.biolang/plugins/<name>/`
- Supported kinds: `python`, `r`, `deno`, `typescript`, `native`
- CLI: `bl add`, `bl remove`, `bl plugins`
