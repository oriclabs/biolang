# Day 22: Language Comparison --- Reproducible Pipelines

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| Load & validate config | 14 | 28 | 30 |
| SHA-256 checksum helper | 2 | 7 | 6 |
| FASTQ parsing & filtering | 16 | 22 | 26 |
| GC content computation | (builtin) | 5 | 4 |
| Process sample function | 22 | 30 | 32 |
| Provenance tracking (create/log/finish/save) | 30 | 28 | 30 |
| Main pipeline orchestration | 32 | 38 | 40 |
| Write CSV output | 3 | 8 | 10 |
| **Total script** | **~120** | **~170** | **~185** |

## Key Differences

### Configuration Loading

```
# BioLang — built-in JSON decode, record access
let config = json_decode(read_lines("config.json") |> reduce(|a, b| a + b))
let mq = config.min_quality

# Python — json module, dict access
with open("config.json") as f:
    config = json.load(f)
mq = config["min_quality"]

# R — jsonlite, list access
config <- fromJSON("config.json")
mq <- config$min_quality
```

All three are comparable. BioLang's record dot-access (`config.min_quality`) is slightly cleaner than Python's bracket syntax or R's dollar-sign.

### Checksums

```
# BioLang — single builtin
let checksum = sha256("data/sample.fastq")

# Python — hashlib with chunked reading
h = hashlib.sha256()
with open(path, "rb") as f:
    for chunk in iter(lambda: f.read(8192), b""):
        h.update(chunk)
checksum = h.hexdigest()

# R — digest package
checksum <- digest(file = path, algo = "sha256")
```

BioLang's `sha256()` builtin handles file reading internally. Python requires manual chunked I/O for large files. R's `digest` package is concise but requires installation.

### FASTQ Processing

```
# BioLang — builtin read + quality filter + pipe
let reads = read_fastq(path)
let filtered = reads |> quality_filter(min_qual)
let gc_values = filtered |> map(|r| gc_content(r.seq))

# Python — manual parsing, list comprehensions
reads = parse_fastq(path)  # 15-line custom function
filtered = [r for r in reads if mean(r["qual"]) >= min_qual]
gc_values = [gc_content(r["seq"]) for r in filtered]

# R — manual parsing, Filter/sapply
reads <- parse_fastq(path)  # 15-line custom function
filtered <- Filter(function(r) mean(r$qual) >= min_qual, reads)
gc_values <- sapply(filtered, function(r) gc_content(r$seq))
```

BioLang's `read_fastq()`, `quality_filter()`, and `gc_content()` are domain-specific builtins that eliminate boilerplate. Python and R both require custom FASTQ parsers and manual GC calculation.

### Provenance Tracking

All three languages require similar amounts of code for provenance tracking — this is general-purpose record manipulation rather than domain-specific work. BioLang's record syntax is slightly more compact, but the logic is equivalent.

### Pipeline Orchestration

```
# BioLang — pipe-based, functional
let results = config.input_files |> map(|f| process_sample(f, config))
let summary = results |> map(|r| {file: r.file, ...}) |> to_table()
summary |> write_csv(output_path)

# Python — loop-based
results = []
for f in config["input_files"]:
    results.append(process_sample(f, config))
# ... DictWriter setup ...

# R — loop + data.frame construction
results <- list()
for (f in config$input_files) {
    results <- c(results, list(process_sample(f, config)))
}
summary_df <- data.frame(...)
write.csv(summary_df, output_path)
```

BioLang's pipe + map pattern is more concise than Python's or R's loop-and-accumulate pattern. The `to_table() |> write_csv()` chain replaces manual CSV writer setup.

## Summary

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| Config loading | `json_decode()` builtin | `json.load()` stdlib | `fromJSON()` (jsonlite) |
| File checksums | `sha256()` builtin | `hashlib` (manual chunking) | `digest()` package |
| FASTQ parsing | `read_fastq()` builtin | Custom function | Custom function |
| Quality filtering | `quality_filter()` builtin | List comprehension | `Filter()` |
| GC content | `gc_content()` builtin | Custom function | Custom function |
| CSV output | `to_table() \|> write_csv()` | `csv.DictWriter` | `write.csv()` |
| JSON output | `json_encode()` builtin | `json.dump()` stdlib | `toJSON()` (jsonlite) |
| Provenance | Manual (all languages similar) | Manual | Manual |
| External deps | None | None (all stdlib) | jsonlite, digest, logging |

BioLang's advantage is concentrated in the bioinformatics-specific operations (FASTQ I/O, quality filtering, GC content, checksums). The general-purpose provenance code is similar across all three languages.
