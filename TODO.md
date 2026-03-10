# BioLang TODO

## Priority 1: Publication-Ready (Required for JOSS/Journal Submission)

### Public Release
- [ ] Publish to GitHub with proper README, LICENSE (MIT/Apache-2.0), and CITATION.cff
- [ ] `cargo install biolang` working from crates.io
- [ ] Pre-built binaries for Linux/macOS/Windows via GitHub Releases
- [ ] Homebrew formula (`brew install biolang`)
- [ ] Version pinned at 1.0.0 with semantic versioning commitment

### Real-World Benchmarks
- [ ] Pick 5 common tasks: FASTQ QC, VCF filtering, FASTA stats, k-mer counting, CSV wrangling
- [ ] Implement each in BioLang, Python+Biopython, R+Bioconductor
- [ ] Measure: wall-clock time, lines of code, memory usage, time-to-first-result
- [ ] Run on real public datasets (SRA accessions for reproducibility)
- [ ] Publish benchmark scripts in `benchmarks/` directory

### End-to-End Case Studies (3 minimum)
- [ ] RNA-seq QC pipeline: FASTQ → quality filter → adapter stats → summary report
- [ ] Variant annotation: VCF → filter PASS → annotate via ClinVar/UniProt → clinical table
- [ ] Sequence exploration: FASTA → GC content → k-mer spectrum → motif search → plot
- [ ] Each case study must run on real data, not simulated tables
- [ ] Document each as a tutorial with expected output

### Beta Users
- [ ] Recruit 3-5 bioinformaticians to try BioLang on their own tasks
- [ ] Collect structured feedback (what worked, what broke, what's missing)
- [ ] Fix top 10 usability issues they report
- [ ] Get 1-2 willing to be acknowledged in the paper

### Paper Preparation
- [ ] JOSS paper draft: `paper.md` + `paper.bib` in repo root
- [ ] Figures: LOC comparison chart, benchmark timings, architecture diagram
- [ ] Statement of need: who benefits and why existing tools fall short
- [ ] Community guidelines: CONTRIBUTING.md, CODE_OF_CONDUCT.md, issue templates

---

## Priority 2: Killer Feature — "Bioinformatics Calculator" REPL

### Zero-Friction Interactive Exploration
- [ ] `bl` with no args drops into REPL instantly (< 1 second startup)
- [ ] Paste a raw DNA sequence → auto-detect and wrap as `dna"..."`
- [ ] Paste a FASTA block → auto-parse into record
- [ ] Paste a UniProt/PDB ID → auto-fetch and display summary
- [ ] Tab-completion for all 270+ builtins with signature hints
- [ ] `:help gc_content` shows usage, example, and return type inline

### One-Line Database Queries
- [ ] `ncbi "BRCA1"` shorthand in REPL (no function call syntax needed)
- [ ] `pdb "6LU7"` → instant structure summary with chains and resolution
- [ ] `uniprot "P53_HUMAN"` → name, function, domains, GO terms in one shot
- [ ] Auto-cache API results in `~/.biolang/cache/` with TTL
- [ ] Offline mode: serve from cache when network unavailable

### REPL Quality of Life
- [ ] Syntax-highlighted output (sequences colored by base, tables formatted)
- [ ] `:save` exports last result to CSV/TSV/FASTA automatically
- [ ] `:plot` renders last numeric result as ASCII histogram in terminal
- [ ] `:history` with fuzzy search
- [ ] Startup banner showing version + available databases + cached data size

---

## Priority 3: Teaching & Onboarding

### Bioinformatics Course Package
- [ ] "Learn BioLang in 30 Minutes" tutorial targeting biologists who know basic R/Python
- [ ] 10 graduated exercises: sequence basics → file I/O → API queries → pipelines
- [ ] Rosalind problem solutions as worked examples (first 20 problems)
- [ ] Side-by-side comparisons: "How you'd do this in Python" vs BioLang
- [ ] Downloadable sample datasets (small: < 10MB each) in `examples/sample-data/`

### Playground / Try-It-Online
- [ ] WebAssembly build of BioLang interpreter
- [ ] Browser-based REPL on biolang.dev (no install required)
- [ ] Pre-loaded examples users can edit and run
- [ ] Share-by-URL for code snippets
- [ ] This is the single highest-impact adoption driver

### Documentation Completeness
- [ ] Every builtin has a doc page with: signature, description, example, return type
- [ ] "Cookbook" section: 50 common tasks with copy-paste solutions
- [ ] Error message guide: common errors and how to fix them
- [ ] Migration guides: "Coming from Biopython", "Coming from R/Bioconductor"

---

## Priority 4: Performance & Scaling

### Interpreter Performance
- [ ] Benchmark against Python for pure-compute tasks (k-mer counting, GC calculation)
- [ ] Profile hot paths: environment lookup, value cloning, string allocation
- [ ] Consider arena allocation for Values during interpretation
- [ ] Streaming must handle 100M+ read FASTQ files in constant memory (verify)
- [ ] Parallel `par_map` / `par_filter` actually using rayon thread pool

### JIT Compilation (Cranelift)
- [ ] Feature-gate behind `--features jit`
- [ ] Target hot loops: map/filter/reduce over large collections
- [ ] Benchmark: show 5-10x speedup on numeric-heavy operations
- [ ] Fallback gracefully to interpreter for unsupported constructs

### Large File Handling
- [ ] Verify streaming works for: 50GB FASTQ, 10GB VCF, 5GB BAM
- [ ] Memory profiling with `heaptrack` or equivalent
- [ ] Indexed access for BAM/VCF (via noodles index support)
- [ ] Progress bars for long-running file operations (`:progress on`)

---

## Priority 5: Ecosystem & Interop

### Package Manager
- [ ] `bl add <package>` installs community packages from registry
- [ ] `biolang.toml` manifest for project dependencies
- [ ] Central registry (start with GitHub-based, like Cargo)
- [ ] Versioned dependencies with lock file

### Python/R Interop
- [ ] `py_eval("import pandas; pandas.read_csv('x.csv')")` → BioLang Table
- [ ] `r_eval("DESeq2::results(dds)")` → BioLang Table
- [ ] Or: export BioLang tables as Arrow/Parquet for zero-copy handoff
- [ ] This answers "Can it call DESeq2?" — the most common objection

### Notebook Integration
- [ ] Jupyter kernel for BioLang (bl-kernel)
- [ ] VS Code extension with syntax highlighting + LSP
- [ ] Quarto/RMarkdown code block support (`{biolang}`)

### Plugin Ecosystem
- [ ] Plugin template: `bl init --plugin`
- [ ] Plugin registry with search
- [ ] Community plugins for: BLAST wrapper, alignment (MAFFT/MUSCLE), phylogenetics
- [ ] This fills the gaps in multispecies.html (align, distance_matrix, etc.)

---

## Priority 6: Missing Language Features

### Alignment & Phylogenetics (Most-Requested Gap)
- [ ] `align(seqs)` — call MAFFT/MUSCLE via subprocess, return alignment object
- [ ] `distance_matrix(alignment)` — pairwise distances with model selection
- [ ] `neighbor_joining(dist)` already exists — verify it works end-to-end
- [ ] `phylo_tree(newick)` already exists — verify SVG output
- [ ] `conservation_scores(alignment)` — per-column conservation
- [ ] This makes the multispecies tutorial actually work

### Named/Optional Arguments
- [ ] Consider adding keyword arguments: `histogram(data, bins=30)`
- [ ] Or standardize on options records: `histogram(data, {bins: 30})`
- [ ] Pick ONE pattern and be consistent — current docs mix both
- [ ] Update all documentation to use the chosen pattern

### Error Messages
- [ ] Include source location (file:line:col) in all runtime errors
- [ ] Suggest fixes: "did you mean `sort_by`?" when `sort_bye` is called
- [ ] Type mismatch errors should show expected vs actual type
- [ ] "Function not found" should list similar builtin names

---

## Visualization Enhancements (Lower Priority)

### genome_track Improvements
- [ ] Basic multi-track stacking (2-3 tables rendered vertically aligned)
- [ ] `export_bed()` builtin — write Table to BED format for IGV/UCSC
- [ ] `export_igv_session()` — generate IGV session XML pointing to exported files
- [ ] Do NOT attempt full MapGen/Gviz replication — too complex, users will use IGV

### Interactive Plots
- [ ] SVG output with hover tooltips (gene names on volcano plot points)
- [ ] Click-to-zoom on Manhattan plots
- [ ] Export to PNG via resvg (no browser dependency)

---

## Website Fixes (In Progress)

### Known Documentation Issues
- [x] Remove `desc()` function usage — use `"-colname"` prefix with `arrange()`
- [x] Fix named argument syntax in examples (BioLang uses positional args)
- [x] Fix `pubmed_search` return type (`Record {count, ids}` not `List`)
- [x] Fix FASTA record `.name` → `.id` across examples
- [ ] **tutorials/multispecies.html** — complete rewrite needed (uses fabricated functions)
- [ ] Audit remaining tutorial pages for fabricated function calls
- [ ] Add "requires" comments to examples that need API keys or external data
