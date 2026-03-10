# Sequence I/O Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Statistics | 0.995 | 1.590 | 1.764 | 1.5x | 1.7x |
| FASTQ QC | 1.960 | 3.551 | 4.404 | 1.8x | 2.2x |
| E.coli Genome Stats | 0.176 | 1.081 | 1.354 | 6.1x | 7.6x |
| Human Chr22 Stats | 1.126 | 1.673 | 2.092 | 1.4x | 1.8x |
| GC Content (51 MB) | 0.830 | 2.771 | 2.358 | 3.3x | 2.8x |
| Reverse Complement | 0.860 | 1.357 | 1.747 | 1.5x | 2.0x |

## Output Comparison

### FASTA Statistics

**BioLang**:
```
Sequences: 10000
Total bp: 27494246
Mean length: 2749.4
Median length: 2723.0
Min length: 500
Max length: 5000
Mean GC: 0.5001
N50: 3603
```

**Python**:
```
Sequences: 10000
Total bp: 27494246
Mean length: 2749.4
Median length: 2723.0
Min length: 500
Max length: 5000
Mean GC: 0.5001
N50: 3603
```

**R**:
```
Sequences: 10000
Total bp: 27494246
Mean length: 2749.4
Median length: 2723.0
Min length: 500
Max length: 5000
Mean GC: 0.5001
N50: 3603
```

### FASTQ QC

**BioLang**:
```
Total reads: 100000
Q30 rate: 0.015%
Mean length: 125.0
Min length: 100
Max length: 150
Mean quality: 27.5
Median quality: 27.5
```

**Python**:
```
Total reads: 100000
Q30 rate: 0.015%
Mean length: 125.0
Min length: 100
Max length: 150
Mean quality: 27.50
Median quality: 27.50
```

**R**:
```
Total reads: 100000
Q30 rate: 0.015%
Mean length: 125.0
Min length: 100
Max length: 150
Mean quality: 27.50
Median quality: 27.50
```

### E.coli Genome Stats

**BioLang**:
```
Sequences: 1
Total bp: 4641652
Mean length: 4641652.0
Min length: 4641652
Max length: 4641652
Mean GC: 0.5079
N50: 4641652
```

**Python**:
```
Sequences: 1
Total bp: 4641652
Mean length: 4641652.0
Min length: 4641652
Max length: 4641652
Mean GC: 0.5079
N50: 4641652
```

**R**:
```
Sequences: 1
Total bp: 4641652
Mean length: 4641652.0
Min length: 4641652
Max length: 4641652
Mean GC: 0.5079
N50: 4641652
```

### Human Chr22 Stats

**BioLang**:
```
Sequences: 1
Total bp: 50818468
Mean length: 50818468.0
Mean GC: 0.3622
N50: 50818468
```

**Python**:
```
Sequences: 1
Total bp: 50818468
Mean length: 50818468.0
Mean GC: 0.3622
N50: 50818468
```

**R**:
```
Sequences: 1
Total bp: 50818468
Mean length: 50818468.0
Mean GC: 0.3622
N50: 50818468
```

### GC Content (51 MB)

**BioLang**:
```
Sequences: 1
Mean GC: 0.3622
Min GC: 0.3622
Max GC: 0.3622
```

**Python**:
```
Sequences: 1
Mean GC: 0.47
Min GC: 0.47
Max GC: 0.47
```

**R**:
```
Sequences: 1
Mean GC: 0.3622
Min GC: 0.3622
Max GC: 0.3622
```

### Reverse Complement

**BioLang**:
```
Sequences: 10000
Total bp (reverse complemented): 27494246
Mean length: 2749.4
```

**Python**:
```
Sequences: 10000
Total bp (reverse complemented): 27494246
Mean length: 2749.4
```

**R**:
```
Sequences: 10000
Total bp (reverse complemented): 27494246
Mean length: 2749.4
```

