# Interval Operations Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| BED Interval Overlap | 0.049 | 0.028 | 1.140 | .5x | 23.2x |
| ENCODE Peak Overlap | 0.154 | 2.614 | - | 16.9x | - |

## Output Comparison

### BED Interval Overlap

**BioLang**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

**Python**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

**R**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

### ENCODE Peak Overlap

**BioLang**:
```
H3K27ac peaks (regions): 52455
CTCF peaks (queries): 41952
Total overlaps: 7301
```

**Python**:
```
H3K27ac peaks (regions): 52455
CTCF peaks (queries): 41952
Total overlaps: 7301
```

