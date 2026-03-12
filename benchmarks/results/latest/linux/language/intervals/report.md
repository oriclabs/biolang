# Interval Operations Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| BED Interval Overlap | 0.160 | 0.067 | 1.279 | .4x | 7.9x |
| ENCODE Peak Overlap | 0.363 | 2.574 | - | 7.0x | - |

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

