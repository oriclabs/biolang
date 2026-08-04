# File I/O Benchmark Report

**Category**: language / file_io
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Small (30 KB) | 0.023 | 0.3 | 1.303 | 13x | 56.7x |
| FASTA Medium (4.6 MB) | 0.032 | 0.309 | 1.375 | 9.7x | 43x |
| FASTA Large (51 MB) | 0.103 | 0.498 | 1.545 | 4.8x | 15x |
| FASTQ (26 MB) | 0.548 | 0.952 | 4.491 | 1.7x | 8.2x |
| VCF (2.3 MB) | 0.046 | 0.05 | 0.294 | 1.1x | 6.4x |
| CSV (0.1 MB) | 0.026 | 0.055 | 0.269 | 2.1x | 10.3x |
| FASTA gzipped (1.3 MB) | 0.037 | 0.357 | 1.374 | 9.6x | 37.1x |
| FASTA Large gzipped (10 MB) | 0.158 | 0.578 | 1.718 | 3.7x | 10.9x |
| Write Filtered FASTA | 0.203 | 0.681 | 1.502 | 3.4x | 7.4x |
| GFF3 (1.7 MB) | 0.119 | 0.052 | 0.38 | 0.4x | 3.2x |
| GFF3 Ensembl chr22 | 0.348 | 0.072 | - | 0.2x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| FASTA Small (30 KB) | 3 | 5 | 4 |
| FASTA Medium (4.6 MB) | 3 | 5 | 4 |
| FASTA Large (51 MB) | 3 | 5 | 4 |
| FASTQ (26 MB) | 3 | 5 | 5 |
| VCF (2.3 MB) | 2 | 6 | 3 |
| CSV (0.1 MB) | 3 | 6 | 3 |
| FASTA gzipped (1.3 MB) | 3 | 7 | 4 |
| FASTA Large gzipped (10 MB) | 3 | 7 | 4 |
| Write Filtered FASTA | 6 | 7 | 7 |
| GFF3 (1.7 MB) | 6 | 17 | 7 |
| GFF3 Ensembl chr22 | 6 | 18 | - |

## Output Comparison

### FASTA Small (30 KB)

**BioLang**:
```
Records: 1
Total bp: 29903
```

**Python**:
```
Records: 1
Total bp: 29903
```

**R**:
```
Records: 1
Total bp: 29903
```

### FASTA Medium (4.6 MB)

**BioLang**:
```
Records: 1
Total bp: 4641652
```

**Python**:
```
Records: 1
Total bp: 4641652
```

**R**:
```
Records: 1
Total bp: 4641652
```

### FASTA Large (51 MB)

**BioLang**:
```
Records: 1
Total bp: 50818468
```

**Python**:
```
Records: 1
Total bp: 50818468
```

**R**:
```
Records: 1
Total bp: 50818468
```

### FASTQ (26 MB)

**BioLang**:
```
Records: 100000
Total bp: 12501923
```

**Python**:
```
Records: 100000
Total bp: 12501923
```

**R**:
```
Records: 100000
Total bp: 12501923
```

### VCF (2.3 MB)

**BioLang**:
```
Records: 50000
```

**Python**:
```
Records: 50000
```

**R**:
```
Records: 50000
```

### CSV (0.1 MB)

**BioLang**:
```
Rows: 5000
Columns: 4
```

**Python**:
```
Rows: 5000
Columns: 4
```

**R**:
```
Rows: 5000
Columns: 4
```

### FASTA gzipped (1.3 MB)

**BioLang**:
```
Records: 1
Total bp: 4641652
```

**Python**:
```
Records: 1
Total bp: 4641652
```

**R**:
```
Records: 1
Total bp: 4641652
```

### FASTA Large gzipped (10 MB)

**BioLang**:
```
Records: 1
Total bp: 50818468
```

**Python**:
```
Records: 1
Total bp: 50818468
```

**R**:
```
Records: 1
Total bp: 50818468
```

### Write Filtered FASTA

**BioLang**:
```
Input records: 10000
Filtered records: 6627
Written: 6627
```

**Python**:
```
Input records: 10000
Filtered records: 6627
Written: 6627
```

**R**:
```
Input records: 10000
Filtered records: 6627
Written: 6627
```

### GFF3 (1.7 MB)

**BioLang**:
```
Total features: 22646
Genes: 5000
Exons: 17646
```

**Python**:
```
Total features: 22646
Genes: 5000
Exons: 17646
```

**R**:
```
Total features: 22646
Genes: 5000
Exons: 17646
```

### GFF3 Ensembl chr22

**BioLang**:
```
Total features: 73048
Genes: 505
Exons: 34428
```

**Python**:
```
Total features: 73048
Genes: 505
Exons: 34428
```

