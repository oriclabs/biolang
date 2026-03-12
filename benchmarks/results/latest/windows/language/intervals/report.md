# Interval Operations Benchmark Report

**Category**: language / intervals
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| BED Interval Overlap | 1.009 | 1.027 | - | 1x | - |
| ENCODE Peak Overlap | 1.026 | 3.03 | - | 3x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| BED Interval Overlap | 7 | 27 | - |
| ENCODE Peak Overlap | 7 | 34 | - |

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

