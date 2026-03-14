# Day 25: Language Comparison --- Error Handling in Production

## Line Counts

| Operation | BioLang | Python (logging + tenacity) | R (tryCatch + futile.logger) |
|-----------|---------|----------------------------|------------------------------|
| Error classification | 7 | 10 | 9 |
| Error log structure | 14 | 24 | 20 |
| Retry logic | 16 | 10 (tenacity decorator) | 12 |
| Input validation | 5 | 10 | 10 |
| FASTQ parsing + error handling | 8 | 18 | 20 |
| Per-file processing | 30 | 35 | 40 |
| Summary generation | 9 | 12 | 14 |
| Directory/file safety | 6 | 3 | 2 |
| Main pipeline loop | 20 | 15 | 16 |
| Output writing | 5 | 12 | 10 |
| **Total script** | **~120** | **~195** | **~185** |

## Key Differences

### try/catch vs try/except vs tryCatch

```
# BioLang --- expression-based, returns a value
let data = try { read_fastq("sample.fastq") } catch err { [] }

# Python --- statement-based, assigns inside block
try:
    data = parse_fastq("sample.fastq")
except Exception as e:
    data = []

# R --- function-based, error handler is a callback
data <- tryCatch(parse_fastq("sample.fastq"), error = function(e) list())
```

### Throwing Errors

```
# BioLang --- error() function
if len(seq) == 0 { error("Empty sequence") }

# Python --- raise statement
if len(seq) == 0:
    raise ValueError("Empty sequence")

# R --- stop() function
if (nchar(seq) == 0) stop("Empty sequence")
```

### Retry Logic

```
# BioLang --- manual loop with range/each
let retry = |f, n| {
    let result = nil
    let ok = false
    range(0, n) |> each(|i| {
        if ok == false {
            try { result = f(); ok = true } catch err { sleep(1000) }
        }
    })
    if ok { result } else { error("Failed") }
}

# Python --- tenacity decorator (3 lines)
@retry(stop=stop_after_attempt(3), wait=wait_exponential())
def read_with_retry(path):
    return parse_fastq(path)

# R --- manual loop (similar to BioLang)
retry_fn <- function(f, max_attempts = 3) {
  for (i in seq_len(max_attempts)) {
    result <- tryCatch(f(), error = function(e) NULL)
    if (!is.null(result)) return(result)
    Sys.sleep(2^(i - 1))
  }
  stop("Failed")
}
```

### Error Logging

```
# BioLang --- append to list, write as CSV table
errors = log_error(errors, ts, file, "ERROR", "Parse failed")
let table = errors |> to_table()
write_csv(table, "error_log.csv")

# Python --- logging module + structured CSV
logger.error(f"{filename}: Parse failed")
writer.writerow({"timestamp": ts, "source": file, ...})

# R --- futile.logger + manual CSV
flog.error("%s: Parse failed", filename)
write.csv(df, "error_log.csv")
```

### Partial Failure Handling

```
# BioLang --- accumulator with each
items |> each(|item| {
    try {
        successes = successes + [process(item)]
    } catch err {
        failures = failures + [{ item: item, error: err }]
    }
})

# Python --- loop with append
for item in items:
    try:
        results.append(process(item))
    except Exception as e:
        failures.append({"item": item, "error": str(e)})

# R --- loop with tryCatch
for (item in items) {
  result <- tryCatch(process(item), error = function(e) NULL)
  if (!is.null(result)) results <- c(results, list(result))
}
```

## Setup Comparison

| Aspect | BioLang | Python | R |
|--------|---------|--------|---|
| Dependencies | None (built-in) | tenacity | jsonlite, futile.logger |
| Error handling | try/catch (expression) | try/except (statement) | tryCatch (function) |
| Retry library | Manual (built-in primitives) | tenacity (decorator) | Manual |
| Logging | Manual CSV via to_table() | logging module (stdlib) | futile.logger |
| File I/O safety | Built-in file_exists() | pathlib (stdlib) | file.exists() (base) |

## Strengths

### BioLang
- try/catch is an expression --- fits inline in assignments and pipes
- No imports needed for error handling, file checks, or JSON
- Accumulator pattern with `each` and list concatenation is concise
- Built-in `quality_filter()` and `gc_content()` reduce per-file processing code

### Python
- tenacity decorator makes retry logic nearly zero-effort
- logging module is mature with handlers, formatters, and log levels
- Exception hierarchy (ValueError, FileNotFoundError, etc.) enables precise catching
- Type hints document error contracts

### R
- tryCatch with `withCallingHandlers` enables warning capture alongside errors
- futile.logger provides hierarchical logging with appenders
- Vectorized operations reduce per-record loop overhead
- Base R file operations need no extra packages
