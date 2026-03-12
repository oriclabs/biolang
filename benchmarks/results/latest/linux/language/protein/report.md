# Protein Analysis Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Protein K-mers | 0.191 | 1.331 | 1.298 | 6.9x | 6.7x |

## Output Comparison

### Protein K-mers

**BioLang**:
```
Proteins: 4531
Total residues: 1387926
Mean length: 306.3
Min length: 8
Max length: 2339
N50: 392
Short (<200 aa): 1566
Medium (200-499 aa): 2369
Long (>=500 aa): 596
```

**Python**:
```
Proteins: 4531
Total residues: 1387926
Mean length: 306.3
Min length: 8
Max length: 2339
N50: 392
Short (<200 aa): 1566
Medium (200-499 aa): 2369
Long (>=500 aa): 596
```

**R**:
```
Proteins: 4531
Total residues: 1387926
Mean length: 306.3
Min length: 8
Max length: 2339
N50: 392
Short (<200 aa): 1566
Medium (200-499 aa): 2369
Long (>=500 aa): 596
```

