# Data Wrangling Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| CSV Join + Group-by | 0.281 | 0.156 | 0.312 | .5x | 1.1x |

## Output Comparison

### CSV Join + Group-by

**BioLang**:
```
Cohort Summary:
  treatment_C: n=1244, depth=42.9, qual=27.6, reads=6438309898
  treatment_B: n=1233, depth=42.8, qual=27.4, reads=6187278991
  treatment_A: n=1268, depth=42.4, qual=27.5, reads=6439472843
  control: n=1255, depth=42.0, qual=27.5, reads=6467124256

High-quality samples: 1618 / 5000
```

**Python**:
```
Cohort Summary:
  treatment_C: n=1244, depth=42.9, qual=27.6, reads=6438309898
  treatment_B: n=1233, depth=42.8, qual=27.4, reads=6187278991
  treatment_A: n=1268, depth=42.4, qual=27.5, reads=6439472843
  control: n=1255, depth=42.0, qual=27.5, reads=6467124256

High-quality samples: 1618 / 5000
```

**R**:
```
Cohort Summary:
  treatment_C: n=1244, depth=42.9, qual=27.6, reads=6438309898
  treatment_B: n=1233, depth=42.8, qual=27.4, reads=6187278991
  treatment_A: n=1268, depth=42.4, qual=27.5, reads=6439472843
  control: n=1255, depth=42.0, qual=27.5, reads=6467124256

High-quality samples: 1618 / 5000
```

