# Day 10: Language Comparison --- Table Operations

## Line Counts

| Operation | BioLang | Python (pandas) | R (dplyr/tidyr) |
|-----------|---------|-----------------|-----------------|
| Load CSV | 1 | 1 | 1 |
| Filter rows | 1 | 1 | 1 |
| Add column (mutate) | 1 | 1 | 1 |
| Group + summarize | 5 | 5 | 5 |
| Left join | 1 | 1 | 1 |
| Pivot wider | 1 | 2 | 2 |
| Pivot longer | 1 | 3 | 2 |
| Write CSV | 1 | 2 | 2 |
| **Total script** | **~65** | **~95** | **~90** |

## Key Differences

### Creating Tables

```
# BioLang
let data = [{gene: "A", val: 1}] |> to_table()

# Python
data = pd.DataFrame([{"gene": "A", "val": 1}])

# R
data <- tibble(gene = "A", val = 1)
```

### Filtering

```
# BioLang — closure with pipe
data |> filter(|r| r.pval < 0.05 and r.log2fc > 1.0)

# Python — boolean indexing
data[(data["pval"] < 0.05) & (data["log2fc"] > 1.0)]

# R — dplyr filter
data %>% filter(pval < 0.05, log2fc > 1.0)
```

### Mutate

```
# BioLang — one column per call, piped
data |> mutate("sig", |r| r.padj < 0.05)

# Python — column assignment
data["sig"] = data["padj"] < 0.05

# R — mutate with tidy eval
data %>% mutate(sig = padj < 0.05)
```

### Group + Summarize

```
# BioLang — explicit callback
data |> group_by("chr") |> summarize(|key, sub| {chr: key, n: nrow(sub)})

# Python — agg dict
data.groupby("chr").agg(n=("gene", "count")).reset_index()

# R — dplyr
data %>% group_by(chr) %>% summarise(n = n())
```

### Pivot

```
# BioLang — positional args
data |> pivot_wider("sample", "expression")
data |> pivot_longer(["S1", "S2"], "sample", "expression")

# Python — different methods
data.pivot(index="gene", columns="sample", values="expression")
data.melt(id_vars=["gene"], var_name="sample", value_name="expression")

# R — tidyr
data %>% pivot_wider(names_from = sample, values_from = expression)
data %>% pivot_longer(cols = c(S1, S2), names_to = "sample", values_to = "expression")
```

### Joins

```
# BioLang — named function
inner_join(a, b, "gene")

# Python — merge method
a.merge(b, on="gene", how="inner")

# R — dplyr
inner_join(a, b, by = "gene")
```

## Observations

- BioLang's `mutate()` adds one column at a time. Chain multiple calls for multiple columns. Python and R can add multiple columns in one call.
- BioLang's `summarize()` uses an explicit callback `|key, subtable|` rather than an aggregation specification. This is more verbose but more flexible --- you can compute arbitrary values per group.
- BioLang's pivot functions use positional arguments. R uses named arguments (`names_from`, `values_from`). Python uses separate method names (`pivot` vs `melt`).
- All three languages support the same set of join types. The syntax is nearly identical.
- BioLang requires no imports or package installation. pandas and dplyr/tidyr must be installed separately.
