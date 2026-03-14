# Day 6: Line-of-Code Comparison

## Task: FASTQ quality control (read stats, filter, trim, k-mer analysis, write output)

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 78 | 118 | 112 |
| Import/setup | 0 | 5 | 3 |
| Read stats | 6 | 12 | 14 |
| Quality analysis | 6 | 6 | 6 |
| Length distribution | 5 | 5 | 5 |
| Standard filtering | 4 | 5 | 5 |
| Custom filtering | 5 | 5 | 3 |
| Quality trimming | 6 | 14 | 4 |
| Adapter detection | 3 | 11 | 11 |
| K-mer analysis | 4 | 8 | 9 |
| Write output | 2 | 4 | 3 |
| Dependencies | 0 (built-in) | 2 (biopython, numpy) | 2 (ShortRead, Biostrings) |

## Key Differences

### Reading FASTQ
```
BioLang:  let reads = read_fastq("file.fastq")
Python:   records = list(SeqIO.parse("file.fastq", "fastq"))
R:        fq <- readFastq("file.fastq")
```

### Per-Read Mean Quality
```
BioLang:  reads |> map(|r| mean_phred(r.qual))
Python:   [np.mean(r.letter_annotations["phred_quality"]) for r in records]
R:        rowMeans(as(quality(fq), "matrix"), na.rm = TRUE)
```

### Filtering Reads
```
BioLang:  reads |> filter_reads(min_length: 50, min_quality: 20)
Python:   [r for r in records if len(r.seq) >= 50
           and np.mean(r.letter_annotations["phred_quality"]) >= 20]
R:        fq[width(sread(fq)) >= 50 & per_read_quality >= 20]
```

### Custom Filter Pipeline
```
BioLang:  reads
              |> filter(|r| len(r.seq) >= 50)
              |> filter(|r| mean_phred(r.qual) >= 20)
              |> filter(|r| gc_content(r.seq) > 0.2 and gc_content(r.seq) < 0.8)
              |> collect()

Python:   [r for r in records
           if len(r.seq) >= 50
           and np.mean(r.letter_annotations["phred_quality"]) >= 20
           and 0.2 < gc_fraction(r.seq) < 0.8]

R:        fq[lengths >= 50 & per_read_quality >= 20
              & gc_fraction > 0.2 & gc_fraction < 0.8]
```

### Quality Trimming
```
BioLang:  let trimmed = trim_quality(reads, min_quality: 20)

Python:   def trim_quality(record, min_qual=20, window=5):  # 8 lines
              ...
          trimmed = [trim_quality(r) for r in records]

R:        trimmed <- trimTails(fq, k = 1, a = "5", successive = TRUE)
```

### K-mer Counting
```
BioLang:  let kmer_freq = kmer_count(first_seq, 5)

Python:   kmers = Counter()
          for i in range(len(seq) - k + 1):
              kmers[seq[i:i+k]] += 1

R:        kmers <- substring(seq, 1:(nchar(seq)-k+1), k:nchar(seq))
          kmer_table <- sort(table(kmers), decreasing = TRUE)
```

### Writing FASTQ
```
BioLang:  write_fastq(clean, "output.fastq")
Python:   SeqIO.write(clean, open("output.fastq", "w"), "fastq")
R:        writeFastq(clean, "output.fastq", compress = FALSE)
```

## Summary

BioLang is the most concise for FASTQ QC. Built-in functions like `read_stats()`, `filter_reads()`, `trim_quality()`, `detect_adapters()`, and `kmer_count()` each replace 5-15 lines of Python/R code. The pipe-based filtering chain reads naturally as a sequential checklist. Python requires Biopython (a large dependency) and manual quality score extraction via `letter_annotations["phred_quality"]`. R requires Bioconductor packages (ShortRead, Biostrings) which have a steep installation overhead. Both Python and R need manual implementation of quality trimming algorithms that BioLang provides as a single function call.
