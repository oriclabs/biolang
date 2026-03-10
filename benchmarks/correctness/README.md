# Correctness Validation

Verifies that BioLang produces the **same biological answers** as Python (BioPython) and R (Bioconductor) on real-world data.

Each task has a BioLang script, a Python script, and (where applicable) an R script that compute the same result. The `validate` script runs all three and compares JSON outputs with appropriate tolerance.

## Tasks

| Task | What it checks | Tolerance | R |
|---|---|---|---|
| `gc_content` | GC% per sequence from FASTA | float ±1e-6 | yes |
| `kmer_count` | Canonical 5-mer counts from DNA | exact integer | -- |
| `vcf_filter` | Filter VCF by QUAL>=30, count per chrom | exact integer | yes |
| `reverse_complement` | Reverse complement of DNA sequences | exact string | yes |
| `translate` | DNA->protein translation | exact string | yes |
| `csv_groupby` | Group-by aggregation (count, mean) | float ±1e-6 | yes |
| `gff_features` | Count features by type from GFF | exact integer | yes |
| `sequence_stats` | N50, total length, GC from FASTA | float ±1e-6 | yes |
| `bed_intervals` | BED parse, span, merge overlapping | exact integer | yes |

## Running

```bash
# Linux/macOS (Python required, R optional)
./validate.sh [bl_binary] [python_binary] [rscript_binary]

# Windows
.\validate.ps1 [-BL bl] [-PY python] [-RS Rscript]
```

If R/Bioconductor is not installed, R tests are skipped automatically.

### R Dependencies

```r
install.packages("BiocManager")
BiocManager::install(c("Biostrings", "GenomicRanges"))
install.packages("jsonlite")
```

## Data

Uses the same `benchmarks/data/` directory as the performance benchmarks.
Run `python benchmarks/generate_data.py` first if data is missing.

## How Comparison Works

1. Python runs first (gold standard), outputs JSON to stdout
2. BioLang runs, outputs identical JSON structure
3. R runs (if available), outputs same JSON
4. A recursive JSON comparator checks all values:
   - Floats: ±1e-6 tolerance
   - Integers: exact match
   - Strings: exact match
   - Dicts/lists: recursive key-by-key comparison
   - Int vs float: allowed if values match within tolerance

## Adding a New Validation

1. Write a Python script in `python/` that prints JSON to stdout
2. Write a BioLang script in `biolang/` that prints identical JSON
3. (Optional) Write an R script in `r/` with the same output
4. Add the task name to the `TASKS` array in `validate.sh` and `validate.ps1`
