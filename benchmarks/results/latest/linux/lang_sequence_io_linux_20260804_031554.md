# Sequence I/O Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Statistics | 0.036 | 0.372 | 1.352 | 10.3x | 37.5x |
| FASTQ QC | 0.831 | 2.306 | 3.833 | 2.7x | 4.6x |
| E.coli Genome Stats | 0.010 | 0.164 | 1.079 | 16.4x | 107.9x |
| Human Chr22 Stats | 0.084 | 0.449 | 1.376 | 5.3x | 16.3x |
| GC Content (51 MB) | 0.125 | 0.721 | 1.409 | 5.7x | 11.2x |
| Reverse Complement | 0.116 | 0.238 | 1.283 | 2.0x | 11.0x |

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

