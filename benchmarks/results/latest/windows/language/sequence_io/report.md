# Sequence I/O Benchmark Report

**Category**: language / sequence_io
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| FASTA Statistics | 1.018 | 1.039 | - | 1x | - |
| FASTQ QC | 2.017 | 3.042 | - | 1.5x | - |
| E.coli Genome Stats | 1.019 | 1.042 | - | 1x | - |
| Human Chr22 Stats | 1.013 | 1.04 | - | 1x | - |
| GC Content (51 MB) | 1.022 | 1.038 | - | 1x | - |
| Reverse Complement | 1.016 | 1.029 | - | 1x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| FASTA Statistics | 9 | 22 | - |
| FASTQ QC | 13 | 14 | - |
| E.coli Genome Stats | 8 | 22 | - |
| Human Chr22 Stats | 6 | 21 | - |
| GC Content (51 MB) | 9 | 8 | - |
| Reverse Complement | 5 | 6 | - |

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

