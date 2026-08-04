# Data Wrangling Benchmark Report

**Category**: language / wrangling
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| CSV Join + Group-by | 0.071 | 0.077 | 0.529 | 1.1x | 7.5x |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| CSV Join + Group-by | 20 | 35 | 31 |

## Output Comparison

### CSV Join + Group-by

**BioLang**:
```
Cohort Summary:
  treatment_C: n=1244, depth=42.9, qual=27.6, reads=6438309898.0
  treatment_B: n=1233, depth=42.8, qual=27.4, reads=6187278991.0
  treatment_A: n=1268, depth=42.4, qual=27.5, reads=6439472843.0
  control: n=1255, depth=42.0, qual=27.5, reads=6467124256.0

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

