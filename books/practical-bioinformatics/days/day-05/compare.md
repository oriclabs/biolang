# Day 5: Line-of-Code Comparison

## Task: Data structures (lists, records/dicts, tables/DataFrames, sets, intervals, combined analysis)

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 112 | 142 | 148 |
| Import/setup | 0 | 3 | 1 |
| List operations | 12 | 12 | 14 |
| Records/dicts | 14 | 16 | 16 |
| Table operations | 20 | 26 | 24 |
| Set operations | 10 | 10 | 10 |
| Interval queries | 10 | 14 | 16 |
| Combined analysis | 18 | 22 | 24 |
| Dependencies | 0 (built-in) | 1 (pandas) | 1 (dplyr) |

## Key Differences

### Creating Records / Dicts / Lists
```
BioLang:  let gene = {symbol: "BRCA1", start: 43044295}
Python:   gene = {"symbol": "BRCA1", "start": 43044295}
R:        gene <- list(symbol = "BRCA1", start = 43044295)
```

### Field Access
```
BioLang:  gene.symbol
Python:   gene["symbol"]
R:        gene$symbol
```

### Has Key
```
BioLang:  has_key(gene, "strand")
Python:   "strand" in gene
R:        "strand" %in% names(gene)
```

### Creating Tables
```
BioLang:  [{gene: "A", pval: 0.01}, {gene: "B", pval: 0.5}] |> to_table()
Python:   pd.DataFrame([{"gene": "A", "pval": 0.01}, {"gene": "B", "pval": 0.5}])
R:        data.frame(gene = c("A", "B"), pval = c(0.01, 0.5))
```

### Table Filter + Sort Pipeline
```
BioLang:  results |> filter(|r| r.pval < 0.05) |> arrange("log2fc")
Python:   results[results["pval"] < 0.05].sort_values("log2fc")
R:        results %>% filter(pval < 0.05) %>% arrange(log2fc)
```

### Set Operations
```
BioLang:  intersection(set_a, set_b)
Python:   set_a & set_b
R:        intersect(vec_a, vec_b)
```

### Interval Overlap Queries
```
BioLang:  let tree = interval_tree(regions)
          query_overlaps(tree, "chr17", 43125300, 43125400)

Python:   [r for r in regions if r["chrom"] == q_chrom
           and r["start"] < q_end and r["end"] > q_start]

R:        regions %>% filter(chrom == q_chrom & start < q_end & end > q_start)
```

### Group By + Summarize
```
BioLang:  results |> group_by("direction") |> summarize(|dir, rows| {direction: dir, count: len(rows)})
Python:   results.groupby("direction").size().reset_index(name="count")
R:        results %>% group_by(direction) %>% summarize(count = n())
```

## Summary

BioLang is the most concise for this task. Sets and interval trees are built-in (zero imports). Table operations use a consistent pipe-based API that reads left-to-right. Python requires pandas for tables and manual loops for interval overlap queries (or pybedtools as an additional dependency). R with dplyr has similar pipe-based flow for tables, but sets are plain vectors (no dedicated type), and interval overlaps require GenomicRanges (Bioconductor) for production use. BioLang's `interval_tree` and `query_overlaps` provide bedtools-level functionality without any external tools.
