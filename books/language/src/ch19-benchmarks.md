# Benchmarks & Correctness

BioLang is benchmarked against Python (BioPython) and R (Bioconductor) on 32 bioinformatics tasks spanning sequence I/O, k-mer analysis, interval overlaps, variant processing, and multi-step pipelines. All results are reproducible from the `benchmarks/` directory.

Every number below comes from `benchmarks/results/latest/linux/scores.yaml`, which the benchmark runner writes. If a figure here disagrees with that file, the file is right.

## Test Environment

### Linux (WSL2)

- Intel Core i9-12900K, 15 GB RAM, Linux 6.6.87
- BioLang 1.1.0, Python 3.12.3, R 4.4.2
- Measured 2026-08-04

### Windows 11

- Intel Core i9-12900K, 32 GB RAM
- BioLang 1.1.0, Python 3.13.14, R 4.6.1
- Measured 2026-08-04

## Read the times before the ratios

These are **whole-script** times: interpreter startup and library imports included, because that is what a one-off analysis actually costs. `import Bio` alone takes 400 ms or more, and BioLang starts in single-digit milliseconds.

That framing flatters BioLang on small inputs, and the effect is large enough to change which numbers mean anything:

- Tasks under a second improved about 5.5x between 0.2.1 and 1.1.0
- Tasks over a second improved about 2.4x
- The two heaviest — 6.1 s and 10.0 s of k-mer counting — moved about 2%

So a 76x on a 30 KB FASTA is mostly a measurement of Python's import time. The k-mer and GC-content rows, where real work dominates the launch, are the ones worth comparing.

## Results Summary

BioLang is faster on 21 of the 32 tasks.

### Where BioLang Wins

The Rust I/O engine (noodles) and native 2-bit DNA encoding deliver the biggest gains on:

| Task | BioLang | Python | Speedup |
|---|---|---|---|
| ENCODE Peak Overlap | 0.154s | 2.614s | **17.0x** |
| E. coli Genome Stats | 0.010s | 0.164s | **16.4x** |
| Protein K-mers | 0.011s | 0.154s | **14.0x** |
| FASTA Statistics | 0.036s | 0.372s | **10.3x** |
| FASTA gzipped (1.3 MB) | 0.018s | 0.168s | **9.3x** |
| GC Content (51 MB) | 0.125s | 0.721s | **5.8x** |
| FASTQ QC Pipeline | 1.024s | 3.630s | **3.5x** |
| K-mer Counting (21-mers) | 6.119s | 19.908s | **3.3x** |
| Chr22 21-mer Count | 9.960s | 28.262s | **2.8x** |

The last two are the load-bearing rows. They run long enough that startup is a rounding error, and they are the ones to cite.

### Where Python Wins

Python's `csv`, `re` and `dict` are heavily optimised C extensions, and on text-heavy parsing they win outright:

| Task | BioLang | Python | Result |
|---|---|---|---|
| GFF3 Ensembl chr22 | 0.295s | 0.036s | Py 8.2x faster |
| GFF3 (1.7 MB) | 0.087s | 0.016s | Py 5.4x faster |
| ClinVar + Ensembl Annotation | 0.095s | 0.034s | Py 2.8x faster |
| ClinVar Variants | 0.135s | 0.064s | Py 2.1x faster |
| BED Interval Overlap | 0.049s | 0.028s | Py 1.8x faster |
| CSV Join + Group-by | 0.052s | 0.033s | Py 1.6x faster |
| VCF (2.3 MB) | 0.019s | 0.014s | Py 1.4x faster |

GFF3 parsing is the clearest gap and worth naming: on Ensembl chr22, BioLang is eight times slower than Python. That is a real weakness in the GFF path, not a measurement artefact.

### Windows

BioLang is faster on **26 of the 32** tasks on Windows, against 21 on Linux. That is not because Windows suits it better; it is because Python's interpreter startup costs more there (0.300s against 0.153s for the same small parse), so several tasks Python narrowly won on Linux go the other way. It is the same startup effect as above, pointing the other direction.

| Task | BioLang | Python | R | Speedup |
|---|---|---|---|---|
| ENCODE Peak Overlap | 0.193s | 3.133s | -- | **16.2x** |
| E. coli Genome Stats | 0.034s | 0.344s | 1.40s | **10.1x** |
| GC Content (51 MB) | 0.133s | 0.940s | 1.72s | **7.1x** |
| FASTQ QC Pipeline | 1.035s | 5.039s | -- | **4.9x** |
| K-mer Counting | 8.390s | 28.768s | -- | **3.4x** |
| Chr22 21-mer Count | 13.849s | 27.594s | -- | **2.0x** |
| GFF3 Ensembl chr22 | 0.348s | 0.072s | -- | Py 4.8x faster |

The GFF3 weakness reproduces on both platforms, which is the useful thing about running the suite twice.

Earlier editions of this chapter said Windows adds roughly a second of process-creation overhead per invocation. That was wrong, and it was a statement about the benchmark runner rather than about Windows. `run_all.ps1` launched every command through PowerShell's `Start-Process -Wait`, which costs about 987 ms: `bl --version` measures 1.012s through it and 0.025s through `System.Diagnostics.Process`. The cost applied to BioLang and Python equally, so it favoured neither — it buried both, which is why every sub-second task used to report `~1.0x`. The runner now starts processes directly. Windows process creation is genuinely dearer than Linux, by tens of milliseconds: BioLang's own startup goes from about 2 ms to 23 ms, which is why the 30 KB FASTA parse reads 13x here and 76x on Linux.

## Code Conciseness

BioLang scripts average 50-70% fewer lines of code than equivalent Python for the same analysis task. This comes from pipe-first syntax, built-in bio types, and higher-order functions on streams.

## Correctness Validation

Performance without correctness is meaningless. BioLang includes two correctness validation suites — synthetic and real-world — that compare outputs against Python (BioPython) and R (Bioconductor) as independent gold standards.

### Synthetic Data Validation

Uses generated test data with controlled inputs for deterministic comparison:

| Task | What it checks | Tolerance | R |
|---|---|---|---|
| `gc_content` | GC% per sequence from FASTA | float ±1e-6 | yes |
| `kmer_count` | Canonical 5-mer counts from DNA | exact integer | -- |
| `vcf_filter` | Filter VCF by QUAL>=30, count per chrom | exact integer | yes |
| `reverse_complement` | Reverse complement of DNA sequences | exact string | yes |
| `translate` | DNA→protein translation | exact string | yes |
| `csv_groupby` | Group-by aggregation (count, mean) | float ±1e-6 | yes |
| `gff_features` | Count features by type from GFF | exact integer | yes |
| `sequence_stats` | N50, total length, GC from FASTA | float ±1e-6 | yes |
| `bed_intervals` | BED parse, span, merge overlapping | exact integer | yes |

### Real-World Data Validation

Uses actual biological data from NCBI and ClinVar to test edge cases that synthetic data misses — non-standard bases, multi-allelic variants, overlapping bacterial genes, and variable naming conventions:

| Task | Real Data Source | Tolerance | R |
|---|---|---|---|
| `gc_content` | S. cerevisiae genome (16 chromosomes) | float ±1e-6 | yes |
| `kmer_count` | E. coli K-12 genome (50 KB) | exact integer | -- |
| `vcf_filter` | ClinVar VCF (5,000 variants, pathogenic filter) | exact integer | yes |
| `reverse_complement` | S. cerevisiae (5 chroms, 200bp each) | exact string | yes |
| `translate` | S. cerevisiae (3 chroms, 99bp each) | exact string | yes |
| `csv_groupby` | ClinVar variants CSV (group by significance) | float ±1e-6 | yes |
| `gff_features` | E. coli K-12 GFF3 annotation | exact integer | yes |
| `sequence_stats` | S. cerevisiae genome | float ±1e-6 | yes |
| `bed_intervals` | E. coli gene BED (derived from GFF) | exact integer | yes |

Real-world data is downloaded automatically via `python download_real_data.py` (~25 MB total from NCBI FTP).

### How It Works

Each task has three implementations — BioLang, Python, and R — that compute the same result and output JSON to stdout. A recursive comparator checks:

- **Floats**: ±1e-6 tolerance
- **Integers**: exact match
- **Strings**: exact match
- **Dicts/lists**: recursive key-by-key comparison

### Running Validation

```bash
# Synthetic data validation
cd benchmarks/correctness
./validate.sh [bl_binary] [python_binary] [rscript_binary]

# Real-world data validation
python download_real_data.py
./validate_real.sh [bl_binary] [python_binary] [rscript_binary]

# Windows
.\validate.ps1 [-BL bl] [-PY python] [-RS Rscript]
.\validate_real.ps1 [-BL bl] [-PY python] [-RS Rscript]
```

R tests are skipped automatically if R/Bioconductor is not installed.

## Reproducing Benchmarks

```bash
# Generate synthetic test data
python benchmarks/generate_data.py

# Run all benchmarks (Linux)
cd benchmarks && ./run_all.sh

# Run correctness validation (synthetic)
cd benchmarks/correctness && ./validate.sh

# Run correctness validation (real-world)
python download_real_data.py && ./validate_real.sh
```

Results are saved to `benchmarks/results/latest/{linux,windows}/` with per-category breakdown:

- `language/` — sequence I/O, k-mers, protein, intervals, variants, file I/O, data wrangling
- `pipelines/` — QC pipeline, variant pipeline, annotation, multi-sample, RNA-seq

## Methodology

- **Timing**: Best of 3 wall-clock runs
- **Data**: Mix of synthetic (generated) and real-world (NCBI, ClinVar, ENCODE, Ensembl)
- **K-mers**: BioLang uses canonical (strand-agnostic) 21-mers; Python uses forward-only — BioLang does strictly more work
- **Fair comparison**: Same input files, same output format, same machine, cold cache between runs
- **Correctness**: Two independent validation suites (synthetic + real-world) ensure identical biological answers
