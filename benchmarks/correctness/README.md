# Correctness Validation

Verifies that BioLang produces the **same biological answers** as Python
(BioPython) and R (Bioconductor).

Rosalind answers "does this match a published answer". This answers the other
half: does it agree with the tools people already use. Each task is written three
times, once per language, and a recursive comparator checks the JSON they emit —
floats to 1e-6, integers and strings exactly.

Agreement is strong evidence, not proof. It adopts BioPython's and Bioconductor's
conventions as the reference, and a shared misreading of a file format would be
invisible to it.

## Three suites

| Suite | Cases | Data | Runner |
|---|---|---|---|
| Synthetic | 14 vs Python, 12 vs R | `benchmarks/data/`, generated | `validate.sh` / `validate.ps1` |
| Real data | 9 vs Python, 8 vs R | NCBI, ClinVar, ENCODE | `validate_real.sh` / `validate_real.ps1` |
| One-liners | 48 vs Python, 47 vs R | none, literal inputs | `oneliners/run_oneliners.py` |

### Synthetic and real-data tasks

| Task | What it checks | Tolerance | R |
|---|---|---|---|
| `gc_content` | GC% per sequence from FASTA | float 1e-6 | yes |
| `kmer_count` | Canonical 5-mer counts from DNA | exact integer | -- |
| `vcf_filter` | Filter VCF, count per chromosome | exact integer | yes |
| `reverse_complement` | Reverse complement of DNA sequences | exact string | yes |
| `translate` | DNA to protein translation | exact string | yes |
| `csv_groupby` | Group-by aggregation (count, mean) | float 1e-6 | yes |
| `gff_features` | Count features by type from GFF | exact integer | yes |
| `sequence_stats` | N50, total length, GC from FASTA | float 1e-6 | yes |
| `bed_intervals` | BED parse, span, merge overlapping | exact integer | yes |
| `edit_distance` | Levenshtein, pairwise over 5 sequences | exact integer | yes |
| `alignment` | Needleman-Wunsch score, match 1 / mismatch -1 / gap -2 | exact integer | yes |
| `diversity` | Shannon and Simpson over feature types | float 1e-6 | yes |
| `hardy_weinberg` | Expected genotype counts from observed | float 1e-6 | yes |
| `peptide_mass` | Integer amino acid mass sum | exact integer | -- |

`edit_distance` is the most useful of these: BioLang uses Myers' bit-parallel
algorithm, Python a dynamic-programming table, and R base `adist()`. Three
different algorithms for one definition.

The last five are synthetic only.

### One-liners

Covering builtins one file at a time does not scale — roughly 214 of the 1018
builtins have a Python or R equivalent, which at three files each would be six
hundred files. The one-liner cases live in
[`oneliners/cases.tsv`](oneliners/cases.tsv), one row per behaviour giving the
same computation in all three languages, and the runner generates a single script
per language. Adding coverage costs a line.

Covered so far: GC content, reverse complement, complement, transcription,
translation, Hamming distance, edit distance, melting temperature, mean, median,
standard deviation, variance, sum, min, max, abs, sqrt, log, pow, floor, ceil,
string case and trimming, substring, prefix and substring tests, split, join,
replace, list length, sort, reverse, unique, and k-mer counts.

## Running

```bash
# Linux/macOS (Python required, R optional)
./validate.sh [bl_binary] [python_binary] [rscript_binary]

# Windows
.\validate.ps1 [-BL bl] [-PY python] [-RS Rscript]

# One-liners, either platform
python oneliners/run_oneliners.py --md results/oneliners.md
```

If R/Bioconductor is not installed, R comparisons are skipped automatically.

### R dependencies

```r
install.packages("BiocManager")
BiocManager::install(c("Biostrings", "GenomicRanges", "pwalign"))
install.packages("jsonlite")
```

`pairwiseAlignment` moved from Biostrings to `pwalign` in Bioconductor 3.19.

## Data

The synthetic suite uses `benchmarks/data/`. Run
`python benchmarks/generate_data.py` first if it is missing.

The real-data suite downloads to `benchmarks/correctness/real_data/`, which is
not in the repository:

```bash
python download_real_data.py    # ~25 MB from NCBI
./validate_real.sh
```

See [`real_world/README.md`](real_world/README.md) for the data sources.

## Evidence

Every run writes to `results/`: `synthetic.{md,json}`, `real-world.{md,json}`
and `oneliners.{md,json}`.

Each records the three tool versions, the data source, the tolerance, and for
every task both a SHA-256 over the canonicalised output and an excerpt of it.
The digests are the point — identical digests are a checkable claim that both
implementations produced the same values, where a bare PASS is only an assertion
that somebody checked. Full outputs are not embedded: `gc_content` alone is
268 KB and `reverse_complement` 27 MB.

The one-liner report carries the actual BioLang, Python and R value for every
case side by side, which is affordable there because the values are scalars.

## Known convention differences

Some disagreements are real and neither side is wrong. These are recorded rather
than hidden: a case marked `differ` in `cases.tsv` does not count as a failure,
but it *does* fail if it ever starts agreeing, so the note cannot go stale.

| Case | BioLang | Python | R | Why |
|---|---|---|---|---|
| `round(2.5)` | 3 | 2 | 2 | BioLang rounds half away from zero; Python and R round half to even |
| `round(2.675, 2)` | 2.68 | 2.67 | 2.67 | Same rule, reached through binary representation |

Two others are handled by making all sides answer the same question, rather than
by recording a divergence:

- **Translation.** BioLang's `translate()` ends at the first stop codon;
  BioPython and Biostrings emit `*` and continue. The references are told to
  stop.
- **Peptide mass.** `peptide_mass()` sums the *integer* amino acid masses used in
  cyclopeptide sequencing (G 57, A 71 ... W 186), not monoisotopic masses.
  SKADYEK is 821 on that table and 821.392 monoisotopic. The reference
  transcribes the same published table rather than calling BioPython, whose
  `molecular_weight()` implements the other convention.

## What is not covered, and why

- **Plot builtins** (20) produce SVG. Comparing rendered output across libraries
  tests the renderer, not the computation.
- **Filesystem, transfer, API and LLM builtins** have no value to compare, and
  anything hitting a live endpoint would only show that both sides parsed the
  same response.
- **Single-cell methods** are stochastic; two implementations can differ while
  both are correct.
- **Runtime builtins** (615) are language internals that Python and R have no
  opinion about.

## Adding a case

For a one-liner, add a row to `oneliners/cases.tsv`. For anything needing data
or setup:

1. Write a Python script in `python/` that prints JSON to stdout.
2. Write a BioLang script in `biolang/` that prints the same structure.
3. Optionally write an R script in `r/`.
4. Add the task name to `$Tasks` in `validate.ps1` and `TASKS` in `validate.sh`.

Run it before committing. Every task in this directory was once written without
being run, and every one of them was broken.
