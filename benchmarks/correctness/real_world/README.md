# Real-World Correctness Validation

Validates BioLang against Python (BioPython) and R (Bioconductor) on **real biological data** from NCBI, ClinVar, and ENCODE.

## Data Sources

| File | Source | Description |
|---|---|---|
| `ecoli_genome.fa` | NCBI RefSeq GCF_000005845.2 | E. coli K-12 MG1655 complete genome (~4.6 MB) |
| `yeast_genome.fa` | NCBI RefSeq GCF_000146045.2 | S. cerevisiae S288C genome, 16 chromosomes (~12 MB) |
| `clinvar.vcf` | NCBI ClinVar GRCh38 | First 5,000 variant records from ClinVar |
| `ecoli_annotation.gff` | NCBI RefSeq GCF_000005845.2 | E. coli K-12 GFF3 annotation (~2 MB) |
| `ecoli_genes.bed` | Derived from GFF | E. coli gene coordinates in BED format |
| `clinvar_variants.csv` | Derived from VCF | ClinVar variants with chrom, pos, ref, alt, clnsig, gene, var_len |

## Tasks

| Task | Data | What it validates | Tolerance | R |
|---|---|---|---|---|
| `gc_content` | Yeast genome (16 chroms) | GC% per chromosome | float ±1e-6 | yes |
| `kmer_count` | E. coli genome (50 KB) | Canonical 5-mer counts | exact integer | -- |
| `vcf_filter` | ClinVar VCF | Filter pathogenic, count per chrom | exact integer | yes |
| `reverse_complement` | Yeast genome (5 seqs, 200bp) | Reverse complement | exact string | yes |
| `translate` | Yeast genome (3 seqs, 99bp) | DNA→protein translation | exact string | yes |
| `csv_groupby` | ClinVar variants CSV | Group by clinical significance | float ±1e-6 | yes |
| `gff_features` | E. coli GFF3 | Count features by type | exact integer | yes |
| `sequence_stats` | Yeast genome | N50, total length, GC | float ±1e-6 | yes |
| `bed_intervals` | E. coli genes BED | Parse, span, merge overlapping | exact integer | yes |

## Running

```bash
# 1. Download real-world data (~25 MB total)
python ../download_real_data.py

# 2. Run validation (from benchmarks/correctness/)
./validate_real.sh [bl_binary] [python_binary] [rscript_binary]

# Windows
.\validate_real.ps1 [-BL bl] [-PY python] [-RS Rscript]
```

## Why Real-World Data?

Synthetic data can hide edge cases that real biological data reveals:

- **Non-standard characters** in genome assemblies (N, Y, R, etc.)
- **Multi-allelic variants** and complex INFO fields in ClinVar
- **Overlapping gene annotations** in dense bacterial genomes
- **Variable chromosome naming** conventions (NC_000913.3 vs chr1)
- **Real GC distributions** that stress floating-point precision
