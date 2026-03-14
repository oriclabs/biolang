# Day 4: Line-of-Code Comparison

## Task: Programming fundamentals (variables, loops, functions, pipes, fold-change analysis)

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 98 | 120 | 128 |
| Import/setup | 0 | 2 | 1 |
| Variables | 5 | 5 | 5 |
| Lists/vectors | 4 | 4 | 4 |
| Records/dicts | 7 | 8 | 8 |
| GC content function | 0 (built-in) | 4 | 5 |
| QC loop | 8 | 8 | 10 |
| Fold-change analysis | 14 | 18 | 18 |
| Dependencies | 0 (built-in) | 1 (pandas) | 1 (dplyr) |

## Key Differences

### Variable Declaration
```
BioLang:  let x = 42
Python:   x = 42
R:        x <- 42
```

### Lists
```
BioLang:  let samples = ["A", "B", "C"]
Python:   samples = ["A", "B", "C"]
R:        samples <- c("A", "B", "C")
```

### Records / Dictionaries
```
BioLang:  let exp = {cell_line: "HeLa", dose: 0.5}
Python:   exp = {"cell_line": "HeLa", "dose": 0.5}
R:        exp <- list(cell_line = "HeLa", dose = 0.5)
```

### Field Access
```
BioLang:  experiment.cell_line
Python:   experiment["cell_line"]
R:        experiment$cell_line
```

### GC Content
```
BioLang:  gc_content(dna"ATGCGA")             # built-in, type-safe
Python:   sum(1 for b in seq if b in "GC") / len(seq)  # manual
R:        sum(bases %in% c("G","C")) / length(bases)    # manual
```

### Pipe Syntax
```
BioLang:  sequences |> map(|s| gc_content(s)) |> filter(|r| r > 0.5)
Python:   [gc_content(s) for s in sequences]  # list comprehension (no pipe)
R:        sequences %>% sapply(gc_content) %>% .[. > 0.5]  # magrittr pipe
```

### Fold-Change Pipeline
```
BioLang:  samples |> map(|s| {gene: s.gene, fc: fold_change(s.control, s.treated)})
          |> filter(|r| r.fc > 2.0) |> sort_by(|r| r.fc) |> reverse()

Python:   df["fc"] = round(df["treated"] / df["control"], 2)
          upregulated = df[df["fc"] > 2.0].sort_values("fc", ascending=False)

R:        results %>% mutate(fc = round(treated / control, 2))
          %>% filter(fc > 2.0) %>% arrange(desc(fc))
```

## Summary

BioLang is the most concise for this task. GC content is built-in (zero lines vs 4-5 lines of manual implementation). The pipe operator (`|>`) provides a natural left-to-right data flow that reads like a protocol. Python uses list comprehensions (a different paradigm). R with dplyr has similar pipe-based flow but requires package imports and more verbose syntax for record access.
