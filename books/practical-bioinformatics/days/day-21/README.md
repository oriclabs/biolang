# Day 21: Performance and Parallel Processing

| | |
|---|---|
| **Difficulty** | Intermediate--Advanced |
| **Biology knowledge** | Basic (sequence analysis, FASTQ/FASTA formats) |
| **Coding knowledge** | Intermediate--Advanced (parallelism, async, streaming, profiling) |
| **Time** | ~3--4 hours |
| **Prerequisites** | Days 1--20 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` |

## What You'll Learn

- How to measure and profile BioLang code for performance bottlenecks
- How to use `par_map` and `par_filter` for parallel data processing
- How to use streaming I/O for constant-memory processing of large files
- How to benchmark BioLang against Python and R

## Files

| File | Description |
|------|-------------|
| `init.bl` | Generates test FASTA/FASTQ data files for benchmarking |
| `scripts/analysis.bl` | Complete BioLang performance benchmark pipeline |
| `scripts/analysis.py` | Python equivalent (Biopython, concurrent.futures, multiprocessing) |
| `scripts/analysis.R` | R equivalent (ShortRead, Biostrings, parallel, future) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected console output |

## Quick Start

```bash
# Generate test data
bl run init.bl

# Run the BioLang version
bl run scripts/analysis.bl

# Run the Python version
pip install -r python/requirements.txt
python scripts/analysis.py

# Run the R version
Rscript r/install.R
Rscript scripts/analysis.R
```

## Output Files

After running:

| File | Description |
|------|-------------|
| `data/sample.fastq` | 100,000 synthetic FASTQ reads |
| `data/large_sample.fastq` | 1,000,000 synthetic FASTQ reads |
| `data/sequences.fasta` | 50,000 synthetic FASTA sequences |
| `results/benchmark_results.csv` | Timing results from all three approaches |
