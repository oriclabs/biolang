# QC Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTQ QC Pipeline | 1.024 | 3.630 | - | 3.5x | - |

## Output Comparison

### FASTQ QC Pipeline

**BioLang**:
```
QC Pipeline Results:
  Reads passing filter: 99983
  Mean quality: 27.5
  Mean length: 125.0
  Min length: 100
  Max length: 150
```

**Python**:
```
QC Pipeline Results:
  Reads passing filter: 99983
  Mean quality: 27.5
  Mean length: 125.0
  Min length: 100
  Max length: 150
```

