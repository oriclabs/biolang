# File I/O Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Small (30 KB) | 0.002 | 0.153 | 1.040 | 76.5x | 520.0x |
| FASTA Medium (4.6 MB) | 0.012 | 0.168 | 1.112 | 14.0x | 92.6x |
| FASTA Large (51 MB) | 0.101 | 0.306 | 1.294 | 3.0x | 12.8x |
| FASTQ (26 MB) | 0.562 | 0.985 | 3.876 | 1.7x | 6.8x |
| VCF (2.3 MB) | 0.019 | 0.014 | 0.150 | .7x | 7.8x |
| CSV (0.1 MB) | 0.010 | 0.017 | 0.138 | 1.7x | 13.8x |
| FASTA gzipped (1.3 MB) | 0.018 | 0.168 | 1.168 | 9.3x | 64.8x |
| FASTA Large gzipped (10 MB) | 0.154 | 0.413 | 1.415 | 2.6x | 9.1x |
| Write Filtered FASTA | 0.167 | 0.281 | 1.265 | 1.6x | 7.5x |
| GFF3 (1.7 MB) | 0.087 | 0.016 | 0.205 | .1x | 2.3x |
| GFF3 Ensembl chr22 | 0.295 | 0.036 | - | .1x | - |

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

