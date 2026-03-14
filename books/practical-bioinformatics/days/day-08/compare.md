# Day 8: Line-of-Code Comparison

## Task: Streaming/constant-memory processing of large bioinformatics files

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 98 | 178 | 196 |
| Import/setup | 0 | 8 | 0 |
| Streaming FASTQ reader | 0 (built-in) | 14 | 18 |
| Streaming FASTA reader | 0 (built-in) | 16 | 16 |
| Streaming VCF reader | 0 (built-in) | 14 | 14 |
| Streaming BED reader | 0 (built-in) | 10 | 12 |
| Count operation | 1 | 1 | 10 |
| Filter + count | 3 | 2 | 8 |
| Reduce (mean GC) | 4 | 5 | 10 |
| Chunked processing | 8 | 5 | 10 |
| Tee/inspect pattern | 5 | 8 | 10 |
| QC report | 25 | 30 | 40 |
| Write filtered | 5 | 6 | 12 |
| Dependencies | 0 (built-in) | 0 (stdlib only) | 0 (base R only) |

## Key Differences

### Opening a stream
```
BioLang:  let s = fastq("reads.fastq")
Python:   def fastq_stream(path): ...  # 14-line generator function
R:        con <- file("reads.fastq", "r")  # then manual read-4-lines loop
```

### Filtering a stream
```
BioLang:  stream |> filter(|r| mean_phred(r.qual) >= 20) |> count()
Python:   sum(1 for r in stream if mean_phred(r["qual"]) >= 20)
R:        for (r in chunk) { if (mean_phred(r$qual) >= 20) n <- n + 1L }
```

### Lazy chaining
```
BioLang:  fastq("f.fq") |> filter(...) |> map(...) |> take(10) |> collect()
Python:   list(islice((transform(r) for r in stream if pred(r)), 10))
R:        # No lazy chaining — must use explicit loops with connections
```

### Chunked processing
```
BioLang:  stream_chunks(stream, 100)
Python:   def stream_chunks(it, n): ...  # 6-line helper
R:        read_fastq_chunk(con, 100)     # 12-line helper function
```

### Reduce
```
BioLang:  stream |> map(|r| {gc: gc_content(r.seq), n: 1})
                 |> reduce(|a, b| {gc: a.gc + b.gc, n: a.n + b.n})
Python:   reduce(lambda a, b: {...}, (... for r in stream))
R:        for (r in chunk) { gc_sum <- gc_sum + gc_content(r$seq); gc_n <- gc_n + 1L }
```

### Tee / inspect
```
BioLang:  stream |> tee(|r| print(r.id)) |> filter(...) |> count()
Python:   for r in stream: print(r["id"]); if pred(r): ...  # manual loop
R:        # Same — must break out of the pipeline into an explicit loop
```

## Summary

BioLang's streaming is built into the language: `fastq()`, `fasta()`, `vcf()`, `bed()`, `bam()`, and `gff()` all return lazy streams that compose with `|>`. Python can achieve similar constant-memory processing with generators, but requires writing custom parser generators for each format (14-16 lines each). R has no generator concept; constant-memory processing requires explicit connection management and chunked reading loops, adding 10-18 lines of boilerplate per format. The pipe-based lazy chaining in BioLang (`filter |> map |> take |> collect`) has no direct R equivalent --- every intermediate step requires an explicit loop. For this day's tasks, BioLang uses 45% fewer lines than Python and 50% fewer than R, with the gap coming entirely from built-in streaming parsers and composable lazy operations.
