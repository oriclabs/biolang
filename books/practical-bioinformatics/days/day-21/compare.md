# Day 21: Language Comparison --- Performance and Parallel Processing

## Line Counts

| Operation | BioLang | Python (concurrent.futures) | R (parallel) |
|-----------|---------|----------------------------|--------------|
| Load FASTQ reads | 1 | 2 | 6 |
| Serial GC analysis | 4 | 5 | 6 |
| Parallel GC analysis | 4 | 6 | 8 |
| Streaming GC analysis | 10 | 14 | 18 |
| Serial k-mer counting | 2 | 2 | 2 |
| Parallel k-mer counting | 2 | 4 | 6 |
| Timer setup + elapsed | 2 | 2 | 2 |
| Results table + CSV export | 4 | 6 | 6 |
| **Total script** | **~35** | **~85** | **~95** |

## Key Differences

### Parallel Map

```
# BioLang — drop-in replacement, one word change
let gc_values = reads |> par_map(|r| gc_content(r.seq))

# Python — requires ProcessPoolExecutor, chunksize tuning, serialization
with ProcessPoolExecutor(max_workers=cpu_count()) as pool:
    gc_values = list(pool.map(gc_func, seqs, chunksize=1000))

# R — requires cluster creation, export, teardown
cl <- makeCluster(detectCores())
clusterExport(cl, "gc_content")
gc_values <- parSapply(cl, seqs, gc_content)
stopCluster(cl)
```

BioLang's `par_map` is a single function call. Python requires a context manager and explicit chunking. R requires creating a cluster, exporting functions, and stopping the cluster --- 4 lines of boilerplate for every parallel operation.

### Streaming I/O

```
# BioLang — callback-based streaming
stream_fastq("file.fq", |r| {
    total = total + gc_content(r.seq)
    count = count + 1
})

# Python — iterator-based streaming (SeqIO.parse returns a generator)
for record in SeqIO.parse("file.fq", "fastq"):
    total += gc_content(str(record.seq))
    count += 1

# R — manual 4-line-at-a-time reading
con <- file("file.fq", "r")
while (TRUE) {
    header <- readLines(con, n = 1)
    if (length(header) == 0) break
    seq_line <- readLines(con, n = 1)
    plus_line <- readLines(con, n = 1)
    qual_line <- readLines(con, n = 1)
    # ... process seq_line ...
}
close(con)
```

Python's SeqIO generator is comparable to BioLang's streaming. R has no built-in streaming FASTQ parser, requiring manual line-by-line reading.

### Timing

```
# BioLang — built-in timer functions
let t = timer_start()
# ... work ...
let elapsed = timer_elapsed(t)

# Python — time module
import time
t = time.time()
# ... work ...
elapsed = time.time() - t

# R — proc.time()
t0 <- proc.time()
# ... work ...
elapsed <- (proc.time() - t0)["elapsed"]
```

All three languages have similar timing APIs. BioLang's is slightly more readable.

### Parallel Filter

```
# BioLang — par_filter is a drop-in replacement
let high_gc = reads |> par_filter(|r| gc_content(r.seq) > 0.5)

# Python — no built-in parallel filter; must map then filter
with ProcessPoolExecutor() as pool:
    gc_values = list(pool.map(gc_func, seqs, chunksize=1000))
high_gc = [s for s, g in zip(seqs, gc_values) if g > 0.5]

# R — no built-in parallel filter; must parSapply then subset
cl <- makeCluster(detectCores())
clusterExport(cl, "gc_content")
mask <- parSapply(cl, seqs, function(s) gc_content(s) > 0.5)
stopCluster(cl)
high_gc <- seqs[mask]
```

BioLang has a dedicated `par_filter` function. Python and R must parallelize the predicate separately, then filter in a second step.

## Performance Summary

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Serial GC (100K reads) | ~1.8s | ~2.5s | ~4.1s |
| Parallel GC (100K reads) | ~0.5s | ~3.2s | ~3.8s |
| Streaming GC (100K reads) | ~2.1s | ~3.0s | ~5.2s |
| K-mer serial (50K seqs) | ~3.2s | ~4.8s | ~8.5s |
| K-mer parallel (50K seqs) | ~0.9s | ~2.1s | ~4.2s |
| Parallel overhead | Minimal (threads) | High (process fork + pickle) | High (cluster + serialize) |
| GIL limitation | None (Rust threads) | Yes (requires multiprocessing) | No GIL but single-threaded |

BioLang's parallelism uses native Rust threads with no serialization overhead, making it consistently faster than Python's process-based parallelism and R's cluster-based parallelism. The difference is most pronounced on workloads with small per-element data, where Python/R serialization costs dominate.

## Summary

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| Parallel map | `par_map` (drop-in) | ProcessPoolExecutor | makeCluster + parSapply |
| Parallel filter | `par_filter` (drop-in) | No direct equivalent | No direct equivalent |
| Streaming I/O | `stream_fastq` callback | SeqIO.parse generator | Manual line reading |
| Profiling | `:time`, `:profile` REPL | `time.time()`, cProfile | `proc.time()`, Rprof |
| Async I/O | `async`/`await_all` | asyncio / concurrent.futures | future / promises |
| Parallelism model | Shared-memory threads | Process fork (GIL bypass) | Cluster (fork/socket) |
| Lines of code | ~35 | ~85 | ~95 |
