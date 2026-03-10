# Correctness Validation

Verifies that BioLang produces the **same biological answers** as BioPython on real-world data.

Each task has a BioLang script and a Python script that compute the same result.
The `validate` script runs both and compares outputs with appropriate tolerance.

## Tasks

| Task | What it checks | Tolerance |
|---|---|---|
| `gc_content` | GC% per sequence from FASTA | float ±1e-6 |
| `kmer_count` | Canonical 5-mer counts from DNA | exact integer match |
| `vcf_filter` | Filter VCF by QUAL≥30, count per chrom | exact integer match |
| `reverse_complement` | Reverse complement of DNA sequences | exact string match |
| `translate` | DNA→protein translation | exact string match |
| `csv_groupby` | Group-by aggregation (count, mean) | float ±1e-6 for means |
| `gff_features` | Count features by type from GFF | exact integer match |
| `sequence_stats` | N50, total length, GC from FASTA | float ±1e-6 for GC |

## Running

```bash
# Linux/macOS
./validate.sh

# Windows
.\validate.ps1
```

## Data

Uses the same `benchmarks/data/` directory as the performance benchmarks.
Run `python benchmarks/generate_data.py` first if data is missing.

## Adding a new validation

1. Write a Python script in `python/` that prints JSON to stdout
2. Write a BioLang script in `biolang/` that prints identical JSON to stdout
3. Add the task name to `validate.sh`
