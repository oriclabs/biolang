# File I/O Benchmark Report

**Category**: language / file_io
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Small (30 KB) | 1.018 | 1.028 | - | 1x | - |
| FASTA Medium (4.6 MB) | 1.022 | 1.038 | - | 1x | - |
| FASTA Large (51 MB) | 1.015 | 1.043 | - | 1x | - |
| FASTQ (26 MB) | 1.018 | 2.054 | - | 2x | - |
| VCF (2.3 MB) | 1.017 | 1.027 | - | 1x | - |
| CSV (0.1 MB) | 1.01 | 1.054 | - | 1x | - |
| FASTA gzipped (1.3 MB) | 1.021 | 1.039 | - | 1x | - |
| FASTA Large gzipped (10 MB) | 1.018 | 1.037 | - | 1x | - |
| Write Filtered FASTA | 1.021 | 1.026 | - | 1x | - |
| GFF3 (1.7 MB) | 1.021 | 1.039 | - | 1x | - |
| GFF3 Ensembl chr22 | 1.018 | 1.032 | - | 1x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| FASTA Small (30 KB) | 3 | 5 | - |
| FASTA Medium (4.6 MB) | 3 | 5 | - |
| FASTA Large (51 MB) | 3 | 5 | - |
| FASTQ (26 MB) | 3 | 5 | - |
| VCF (2.3 MB) | 2 | 6 | - |
| CSV (0.1 MB) | 3 | 6 | - |
| FASTA gzipped (1.3 MB) | 3 | 7 | - |
| FASTA Large gzipped (10 MB) | 3 | 7 | - |
| Write Filtered FASTA | 6 | 7 | - |
| GFF3 (1.7 MB) | 6 | 17 | - |
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

### VCF (2.3 MB)

**BioLang**:
```
Records: 50000
```

**Python**:
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

