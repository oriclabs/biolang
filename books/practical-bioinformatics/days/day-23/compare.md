# Day 23: Language Comparison --- Batch Processing and Automation

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| Config loading | 2 | 4 | 3 |
| Sample sheet parsing + validation | 12 | 18 | 22 |
| FASTQ parsing & filtering | (builtin) | 16 | 18 |
| GC content computation | (builtin) | 5 | 4 |
| Process sample function | 22 | 38 | 42 |
| Batch runner with error handling | 16 | 12 | 14 |
| Outlier detection | 14 | 12 | 14 |
| Group summarization | 10 | 16 | 14 |
| Main pipeline orchestration | 30 | 40 | 44 |
| CSV output | 4 | 12 | 10 |
| **Total script** | **~110** | **~175** | **~195** |

## Key Differences

### Sample Sheet Parsing

```
# BioLang — built-in CSV, table operations
let sheet = read_csv("data/sample_sheet.csv")
let files = sheet |> select("fastq_file") |> flatten()

# Python — csv.DictReader
with open(path) as f:
    reader = csv.DictReader(f)
    for row in reader:
        samples.append({...})

# R — read.csv + lapply
df <- read.csv(path, stringsAsFactors = FALSE)
lapply(seq_len(nrow(df)), function(i) { list(...) })
```

BioLang's `read_csv()` returns a table object with named columns. Python's `csv.DictReader` requires manual iteration. R's `read.csv` returns a data frame that must be converted to a list of records for per-sample processing.

### Parallel Execution

```
# BioLang — one-word change from map to par_map
let results = samples |> par_map(|s| process_sample(s, config))

# Python — multiprocessing Pool
with Pool(processes=cpu_count()) as pool:
    results = pool.map(process_sample, args_list)

# R — platform-dependent branching
if (.Platform$OS.type == "windows") {
    cl <- makeCluster(n_cores)
    clusterExport(cl, c("parse_fastq", "gc_content", ...))
    results <- parLapply(cl, samples, function(s) ...)
    stopCluster(cl)
} else {
    results <- mclapply(samples, function(s) ..., mc.cores = n_cores)
}
```

BioLang's `par_map` is the simplest: swap one function name. Python requires wrapping arguments in tuples for `Pool.map`. R requires platform-dependent code — `mclapply` uses fork (Unix only), while `parLapply` (Windows) requires explicit variable export.

### Error Isolation

```
# BioLang — try/catch per sample in each loop
samples |> each(|s| {
    try {
        let result = process_sample(s, config)
        results = results + [result]
    } catch err {
        errors = errors + [{sample_id: s.id, error: str(err)}]
    }
})

# Python — try/except inside the worker function
try:
    ... processing ...
    return {"status": "ok", ...}
except Exception as e:
    return {"status": "error", "error": str(e), ...}

# R — tryCatch inside the worker function
tryCatch({
    ... processing ...
    list(status = "ok", ...)
}, error = function(e) {
    list(status = "error", error_msg = conditionMessage(e), ...)
})
```

All three languages handle per-sample error isolation similarly. Python and R embed the try/except inside the worker function (required for multiprocessing). BioLang wraps the call site, which keeps the processing function clean.

### Group Aggregation

```
# BioLang — pipe with group_by
batch.results |> group_by(|r| r.group) |> map(|entry| {
    group: entry.key,
    mean_pass_rate: entry.values |> map(|r| r.pass_rate) |> mean()
})

# Python — defaultdict grouping
groups = defaultdict(list)
for r in results:
    groups[r["group"]].append(r)

# R — split + lapply
groups <- unique(sapply(results, `[[`, "group"))
lapply(groups, function(g) { ... })
```

BioLang's `group_by` + pipe is the most concise. Python requires a defaultdict accumulation loop. R requires extracting unique keys and filtering for each.

## Summary

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| Sample sheet parsing | `read_csv()` builtin | `csv.DictReader` | `read.csv()` |
| FASTQ parsing | `read_fastq()` builtin | Custom 15-line function | Custom 15-line function |
| Quality filtering | `quality_filter()` builtin | List comprehension | `Filter()` |
| Parallel execution | `par_map()` (1 word) | `multiprocessing.Pool` | `mclapply`/`parLapply` (platform-split) |
| Error isolation | `try/catch` per sample | `try/except` in worker | `tryCatch` in worker |
| Group aggregation | `group_by()` pipe | `defaultdict` loop | `unique()` + `Filter()` |
| CSV output | `to_table() \|> write_csv()` | `csv.DictWriter` (12 lines) | `write.csv()` |
| External deps | None | None (all stdlib) | jsonlite |

BioLang's advantage is most pronounced in three areas: (1) FASTQ I/O and quality filtering are single builtins, (2) parallel execution is a one-word change, and (3) the pipe + group_by pattern replaces manual grouping loops. The total script is roughly 35-45% shorter than Python/R equivalents.
