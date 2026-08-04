# Sequence I/O Benchmark Report

**Category**: language / sequence_io
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Statistics | 0.065 | 0.578 | 1.558 | 8.9x | 24x |
| FASTQ QC | 0.82 | 2.558 | 5.485 | 3.1x | 6.7x |
| E.coli Genome Stats | 0.034 | 0.344 | 1.4 | 10.1x | 41.2x |
| Human Chr22 Stats | 0.096 | 0.66 | 1.758 | 6.9x | 18.3x |
| GC Content (51 MB) | 0.133 | 0.94 | 1.723 | 7.1x | 13x |
| Reverse Complement | 0.159 | 0.481 | 1.598 | 3x | 10.1x |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| FASTA Statistics | 9 | 22 | 16 |
| FASTQ QC | 13 | 14 | 13 |
| E.coli Genome Stats | 8 | 22 | 16 |
| Human Chr22 Stats | 6 | 21 | 14 |
| GC Content (51 MB) | 9 | 8 | 7 |
| Reverse Complement | 5 | 6 | 7 |

## Output Comparison

> **Note on k-mer counts:** BioLang reports slightly fewer distinct k-mers than Python (e.g. 27,294,096 vs 27,294,178). This is expected -- BioLang uses **canonical k-mers** (each k-mer and its reverse complement map to the same key), while Python counts raw forward-strand k-mers only.

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

