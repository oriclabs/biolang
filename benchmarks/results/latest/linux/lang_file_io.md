# File I/O Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Small (30 KB) | 0.138 | 0.926 | 1.243 | 6.7x | 9.0x |
| FASTA Medium (4.6 MB) | 0.170 | 0.991 | 1.371 | 5.8x | 8.0x |
| FASTA Large (51 MB) | 0.821 | 1.649 | 2.103 | 2.0x | 2.5x |
| FASTQ (26 MB) | 1.635 | 2.194 | 4.429 | 1.3x | 2.7x |
| VCF (2.3 MB) | 0.159 | 0.068 | 0.168 | .4x | 1.0x |
| CSV (0.1 MB) | 0.120 | 0.059 | 0.102 | .4x | .8x |
| FASTA gzipped (1.3 MB) | 0.141 | 0.930 | 1.327 | 6.5x | 9.4x |
| FASTA Large gzipped (10 MB) | 0.394 | 1.219 | 1.870 | 3.0x | 4.7x |
| Write Filtered FASTA | 0.821 | 1.679 | 4.924 | 2.0x | 5.9x |
| GFF3 (1.7 MB) | 0.189 | 0.064 | 0.205 | .3x | 1.0x |
| GFF3 Ensembl chr22 | 0.453 | 0.171 | - | .3x | - |

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

