# BioLang Benchmarks

Comparative benchmarks: **BioLang** vs **Python + BioPython** vs **R + Bioconductor**.

30 tasks across 9 categories on synthetic and real-world data (NCBI, UniProt, ClinVar, ENCODE, Ensembl).

## Setup

### 1. Install BioLang

```bash
# From the biolang repo root
cargo install --path crates/bl-cli
bl --version   # should print "bl 0.2.x"
```

### 2. Install Python dependencies

#### Linux / macOS

```bash
cd benchmarks

# Automated (creates .venv, installs biopython)
./setup_python.sh

# — or manual —
python3 -m venv .venv
source .venv/bin/activate
pip install biopython
```

#### Windows (PowerShell)

```powershell
cd benchmarks

# Automated (creates .venv, installs biopython)
powershell -ExecutionPolicy Bypass -File .\setup_python.ps1

# — or manual —
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install biopython
```

**Required packages:**

| Package | PyPI | Used by |
|---|---|---|
| `biopython` | `pip install biopython` | FASTA/FASTQ parsing, sequence ops, protein analysis (most benchmarks) |

Standard library modules used (no install needed): `csv`, `re`, `gzip`, `collections`, `statistics`, `math`.

### 3. Install R dependencies (optional)

#### Linux / macOS

```bash
# Automated (installs BiocManager + all Bioconductor packages)
./setup_r.sh

# — or manual —
Rscript -e 'install.packages("BiocManager", repos="https://cloud.r-project.org")'
Rscript -e 'BiocManager::install(c("Biostrings", "ShortRead", "VariantAnnotation", "GenomicRanges"))'
Rscript -e 'install.packages("dplyr", repos="https://cloud.r-project.org")'
```

#### Windows (PowerShell)

```powershell
# Automated
powershell -ExecutionPolicy Bypass -File .\setup_r.ps1

# — or manual —
Rscript -e "install.packages('BiocManager', repos='https://cloud.r-project.org')"
Rscript -e "BiocManager::install(c('Biostrings', 'ShortRead', 'VariantAnnotation', 'GenomicRanges'))"
Rscript -e "install.packages('dplyr', repos='https://cloud.r-project.org')"
```

**Required R packages:**

| Package | Source | Used by |
|---|---|---|
| `Biostrings` | Bioconductor | FASTA parsing, sequence ops, protein, GC content, reverse complement |
| `ShortRead` | Bioconductor | FASTQ parsing and QC |
| `VariantAnnotation` | Bioconductor | VCF parsing and filtering |
| `GenomicRanges` | Bioconductor | BED interval overlap |
| `dplyr` | CRAN | CSV join and group-by |

R is optional -- the runner skips R benchmarks if `Rscript` is not found, and reports failed R benchmarks as `null` if packages are missing.

### 4. Generate benchmark data

```bash
cd benchmarks   # if not already there

# Synthetic data (~55 MB) — required
python3 generate_data.py

# Real-world data (~70 MB compressed) — recommended
python3 download_data.py
```

### 5. Run benchmarks

```bash
# Linux / macOS (activate venv first if you used setup_python.sh)
source .venv/bin/activate
./run_all.sh

# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -File .\run_all.ps1
```

The runner checks dependencies at startup and warns about missing packages. Failed benchmarks are reported as `FAILED` in the console and `null` in the YAML scores file (not silently timed as 0s).

### WSL / Linux notes

If running in WSL, make sure Python packages are installed **inside WSL**, not in the Windows Python. The runner auto-activates `.venv/bin/activate` if present.

```bash
# Verify biopython is available
python3 -c "from Bio import SeqIO; print('biopython OK')"

# Verify R packages (if using R)
Rscript -e 'library(Biostrings); cat("Biostrings OK\n")'
```

---

## Categories

### Language Benchmarks

| Category | Benchmarks | What It Tests |
|---|---|---|
| **Sequence I/O** | FASTA stats, FASTQ QC, genome stats, chr22 stats | Parsing + basic analysis |
| **K-mer Analysis** | K-mer counting (synthetic), chr22 21-mers (real) | Hash-based counting at scale |
| **Variant Analysis** | VCF filtering, ClinVar pathogenicity | Structured variant processing |
| **Data Wrangling** | CSV join + group-by + summarize | Table operations |
| **Protein Analysis** | Proteome stats, protein k-mers | Protein sequence processing |
| **File I/O** | FASTA (small/medium/large), FASTQ, VCF, CSV, GFF3 | Pure parsing speed |
| **Intervals** | BED overlap (synthetic), ENCODE peak overlap (real) | Genomic interval operations |

### Pipeline Benchmarks

| Category | Stages | What It Tests |
|---|---|---|
| **QC Pipeline** | Read FASTQ -> filter -> stats -> report | Multi-stage pipe chain |
| **Variant Pipeline** | Read VCF -> filter -> classify -> summarize | Pipeline with grouping |
| **Multi-Sample** | Load sheets -> join -> per-cohort stats -> aggregate | Batch processing pattern |
| **RNA-seq Mini** | Count matrix -> normalize -> DE analysis | Expression analysis |
| **Annotation** | Read VCF -> classify -> annotate -> report | Variant annotation workflow |

## Filtering

```bash
# By group
./run_all.sh language              # All language benchmarks
./run_all.sh pipelines             # All pipeline benchmarks

# By category
./run_all.sh kmer                  # K-mer analysis only
./run_all.sh file_io               # File I/O only
./run_all.sh qc                    # QC pipeline only
./run_all.sh variant               # Variant benchmarks (language + pipeline)
```

```powershell
powershell -ExecutionPolicy Bypass -File .\run_all.ps1 kmer
powershell -ExecutionPolicy Bypass -File .\run_all.ps1 file_io
powershell -ExecutionPolicy Bypass -File .\run_all.ps1 pipelines
```

## Directory Structure

```
benchmarks/
  setup_python.sh         # Python dependency installer (creates .venv)
  setup_r.sh              # R/Bioconductor dependency installer
  generate_data.py        # Synthetic data generator
  download_data.py        # Real-world data downloader
  run_all.sh              # Linux/macOS runner
  run_all.ps1             # Windows runner
  language/
    sequence_io/          # FASTA/FASTQ stats, genome analysis
      biolang/  python/  r/
    kmer/                 # K-mer counting and analysis
      biolang/  python/  r/
    variants/             # VCF filtering and annotation
      biolang/  python/  r/
    wrangling/            # CSV joins, group-by, summarize
      biolang/  python/  r/
    protein/              # Protein sequence analysis
      biolang/  python/  r/
    file_io/              # Pure file parsing speed
      biolang/  python/  r/
    intervals/            # BED/ENCODE interval overlap
      biolang/  python/  r/
  pipelines/
    qc_pipeline/          # Multi-stage QC workflow
      biolang/  python/
    variant_pipeline/     # Variant analysis workflow
      biolang/  python/
    multi_sample/         # Batch sample processing
      biolang/  python/
    rnaseq_mini/          # Expression analysis
      biolang/  python/
    annotation/           # Variant annotation
      biolang/  python/
  data/                   # Synthetic data (generate_data.py)
  data_real/              # Real-world data (download_data.py)
  results/                # Output reports and YAML scores
```

## Data

### Synthetic Data

```bash
python3 generate_data.py
```

| File | Size | Contents |
|---|---|---|
| `sequences.fa` | 27 MB | 10,000 random sequences (500-5000 bp, seed=42) |
| `reads.fq` | 26 MB | 100,000 random reads (100-150 bp, Q15-Q40) |
| `variants.vcf` | 2.3 MB | 50,000 random variants (80% SNPs, 20% indels) |
| `samples.csv` | 0.1 MB | 5,000 samples with depth/quality/read_count |
| `metadata.csv` | 0.1 MB | 5,000 samples with cohort/site/age/sex |

### Real-World Data

```bash
python3 download_data.py               # Download all (~70 MB compressed)
python3 download_data.py ecoli         # Just E. coli genome + proteome
python3 download_data.py clinvar       # Just ClinVar VCF
python3 download_data.py chr22         # Just human chr22
python3 download_data.py encode        # ENCODE ChIP-seq peaks (BED)
python3 download_data.py ensembl       # Ensembl GFF3 gene annotations
```

| File | Size | Source |
|---|---|---|
| `sarscov2_genome.fa` | 30 KB | NCBI RefSeq -- SARS-CoV-2 reference |
| `ecoli_genome.fa` | 4.7 MB | NCBI RefSeq -- E. coli K-12 MG1655 |
| `ecoli_proteome.fa` | 1.9 MB | UniProt -- reviewed E. coli K-12 proteins |
| `clinvar_20k.vcf` | 8.7 MB | NCBI ClinVar -- 20,000 clinical variants (GRCh38, chr1) |
| `clinvar_diverse.vcf` | 9.3 MB | NCBI ClinVar -- ~1000 variants/chrom x 24 chromosomes |
| `human_chr22.fa` | 51 MB | NCBI RefSeq -- Human GRCh38 chromosome 22 |
| `encode_h3k27ac_peaks.bed` | ~3 MB | ENCODE -- H3K27ac ChIP-seq peaks (GM12878) |
| `encode_ctcf_peaks.bed` | ~2 MB | ENCODE -- CTCF ChIP-seq peaks (GM12878) |
| `ensembl_chr22.gff3` | ~5 MB | Ensembl -- GRCh38.112 chr22 gene annotations |
| `gene_annotations.csv` | ~50 KB | Derived from Ensembl GFF3 (gene/pathway/biotype) |

## Output

Each run produces:

- `results/scores_<platform>_<timestamp>.yaml` -- **single source of truth** for all benchmark numbers
- `results/summary_<platform>_<timestamp>.md` -- human-readable report with all categories
- `results/lang_<category>_<platform>_<timestamp>.md` -- per-category detail with output comparison
- `results/pipe_<category>_<platform>_<timestamp>.md` -- pipeline detail with output comparison

## Methodology

- **Time**: Wall-clock, best of 3 consecutive runs
- **LOC**: Non-blank, non-comment lines (idiomatic code per language)
- **Data**: Synthetic is reproducible (seed=42); real-world from public databases
- **Conditions**: All tasks run sequentially, no other heavy processes
- **Error handling**: Failed benchmarks (missing dependencies, runtime errors) are reported as `null`, not silently timed
- **K-mer note**: BioLang counts *canonical* (strand-agnostic) 21-mers. Python counts *raw* (forward-only) k-mers. Canonical counting is strictly more work, making BioLang's timing conservative.

## Results

### Linux (WSL2)

*Intel i9-12900K, 16 GB RAM, WSL2 Ubuntu, BioLang 0.2.1, Python 3.12.3, R 4.3.3*

Source: `results/scores_linux_20260310_213601.yaml`

#### Highlights

| Task | BioLang | Python | Speedup |
|---|---|---|---|
| FASTA Small (30 KB) | 0.129s | 1.031s | **8.0x** |
| ENCODE Peak Overlap | 0.333s | 2.534s | **7.6x** |
| FASTA gzipped (1.3 MB) | 0.150s | 1.017s | **6.8x** |
| E.coli Genome Stats | 0.177s | 1.157s | **6.5x** |
| FASTA Medium (4.6 MB) | 0.190s | 1.183s | **6.2x** |
| Protein K-mers | 0.223s | 1.312s | **5.9x** |
| FASTA Statistics | 0.459s | 1.655s | **3.6x** |
| K-mer Counting | 6.380s | 20.24s | **3.2x** |
| FASTA Large gz (10 MB) | 0.434s | 1.345s | **3.1x** |
| Chr22 21-mer Count | 10.27s | 27.12s | **2.6x** |
| GC Content (51 MB) | 0.824s | 2.121s | **2.6x** |
| Human Chr22 Stats | 0.879s | 2.100s | **2.4x** |
| Write Filtered FASTA | 0.820s | 1.809s | **2.2x** |
| FASTQ QC | 1.916s | 3.734s | **1.9x** |
| FASTQ QC Pipeline | 2.527s | 4.857s | **1.9x** |

Python wins on text-heavy parsing where its csv/re modules are optimized C extensions (VCF filtering, CSV wrangling, GFF3 parsing, BED overlap on synthetic data).

50–70% fewer lines of code than Python across all benchmarks.

### Windows 11

*Intel i9-12900K, 32 GB RAM, Windows 11, BioLang 0.2.1, Python 3.12.4, R 4.5.2*

Source: `results/scores_windows_20260310_191450.yaml`

Note: Windows process creation adds ~1s overhead per benchmark invocation, compressing timing differences on fast tasks.

#### Highlights

| Task | BioLang | Python | Speedup |
|---|---|---|---|
| ENCODE Peak Overlap | 1.02s | 3.06s | **3.0x** |
| K-mer Counting | 6.06s | 17.2s | **2.8x** |
| Chr22 21-mer Count | 9.08s | 23.3s | **2.6x** |
| FASTQ QC Pipeline | 2.04s | 4.08s | **2.0x** |
| FASTQ QC | 2.03s | 3.05s | **1.5x** |

Full results with 30 benchmarks and output comparison in `results/`.
