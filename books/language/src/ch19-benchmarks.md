# Benchmarks & Correctness

BioLang is benchmarked against Python (BioPython) and R (Bioconductor) on 30 bioinformatics tasks spanning sequence I/O, k-mer analysis, interval overlaps, variant processing, and multi-step pipelines. All results are reproducible from the `benchmarks/` directory.

## Test Environment

### Linux (WSL2)

- Intel Core i9-12900K, 16 GB RAM
- BioLang 0.3.0, Python 3.12.3, R 4.3.3

### Windows 11

- Intel Core i9-12900K, 32 GB RAM
- BioLang 0.3.0, Python 3.12.4

## Results Summary

### Where BioLang Wins

BioLang's Rust I/O engine (noodles) and native 2-bit DNA encoding deliver the biggest gains on:

| Task | BioLang | Python | Speedup |
|---|---|---|---|
| ENCODE Peak Overlap | 0.363s | 2.574s | **7.1x** |
| Protein K-mers | 0.191s | 1.331s | **7.0x** |
| FASTA Parse (30 KB) | 0.138s | 0.926s | **6.7x** |
| FASTA gzipped | 0.141s | 0.930s | **6.6x** |
| E. coli Genome | 0.176s | 1.081s | **6.1x** |
| GC Content (51 MB) | 0.830s | 2.771s | **3.3x** |
| K-mer Counting (21-mers) | 6.551s | 21.01s | **3.2x** |
| FASTQ QC Pipeline | 2.349s | 5.059s | **2.2x** |

### Where Python Wins

Python's `csv`, `re`, and `dict` modules are highly optimized C extensions. On text-heavy parsing tasks:

| Task | BioLang | Python | Result |
|---|---|---|---|
| VCF Filtering | 0.349s | 0.166s | Py 2.1x faster |
| ClinVar Variants | 0.661s | 0.265s | Py 2.5x faster |
| CSV Join + Group-by | 0.281s | 0.156s | Py 1.8x faster |
| GFF3 Ensembl chr22 | 0.453s | 0.171s | Py 2.6x faster |

### Windows Notes

Windows process creation adds ~1s overhead per invocation, compressing speedup ratios for sub-second tasks. For accurate algorithmic comparison, refer to Linux results. CPU-bound tasks that exceed this floor (k-mer counting 3.0x, ENCODE overlap 2.9x, QC pipeline 2.5x) still show clear wins.

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
