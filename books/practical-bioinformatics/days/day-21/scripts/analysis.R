#!/usr/bin/env Rscript
# Day 21: Performance and Parallel Processing — R equivalent

library(parallel)

gc_content <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  if (length(chars) == 0) return(0.0)
  sum(chars %in% c("G", "C")) / length(chars)
}

count_kmers <- function(seq, k = 6) {
  n <- nchar(seq)
  if (n < k) return(0L)
  length(sapply(1:(n - k + 1), function(i) substr(seq, i, i + k - 1)))
}

read_fastq_simple <- function(path) {
  lines <- readLines(path)
  n <- length(lines) %/% 4
  seqs <- lines[seq(2, length(lines), by = 4)]
  seqs[1:n]
}

read_fasta_simple <- function(path) {
  lines <- readLines(path)
  seqs <- character(0)
  current_seq <- ""
  for (line in lines) {
    if (startsWith(line, ">")) {
      if (nchar(current_seq) > 0) {
        seqs <- c(seqs, current_seq)
      }
      current_seq <- ""
    } else {
      current_seq <- paste0(current_seq, line)
    }
  }
  if (nchar(current_seq) > 0) {
    seqs <- c(seqs, current_seq)
  }
  seqs
}

cat(strrep("=", 60), "\n")
cat("Day 21: Performance Benchmark (R)\n")
cat(strrep("=", 60), "\n")

# --- Load FASTQ ---
cat("\nLoading FASTQ reads...\n")
seqs <- read_fastq_simple("data/sample.fastq")
cat(sprintf("Loaded %d reads\n", length(seqs)))

# --- Serial GC analysis ---
cat("\n-- Serial GC Analysis --\n")
t0 <- proc.time()
gc_values <- sapply(seqs, gc_content, USE.NAMES = FALSE)
avg_gc <- mean(gc_values)
high_gc <- sum(gc_values > 0.5)
lengths <- nchar(seqs)
mean_len <- mean(lengths)
min_len <- min(lengths)
max_len <- max(lengths)
serial_time <- (proc.time() - t0)["elapsed"]

cat(sprintf("  Mean GC:    %.1f%%\n", avg_gc * 100))
cat(sprintf("  High GC:    %d reads\n", high_gc))
cat(sprintf("  Mean len:   %.0f\n", mean_len))
cat(sprintf("  Length range: %d-%d\n", min_len, max_len))
cat(sprintf("  Time:       %.3fs\n", serial_time))

# --- Parallel GC analysis ---
cat("\n-- Parallel GC Analysis --\n")
n_cores <- detectCores()
t0 <- proc.time()
cl <- makeCluster(n_cores)
clusterExport(cl, "gc_content")
gc_values_par <- parSapply(cl, seqs, gc_content)
stopCluster(cl)
avg_gc_par <- mean(gc_values_par)
high_gc_par <- sum(gc_values_par > 0.5)
lengths_par <- nchar(seqs)
mean_len_par <- mean(lengths_par)
min_len_par <- min(lengths_par)
max_len_par <- max(lengths_par)
parallel_time <- (proc.time() - t0)["elapsed"]

cat(sprintf("  Mean GC:    %.1f%%\n", avg_gc_par * 100))
cat(sprintf("  High GC:    %d reads\n", high_gc_par))
cat(sprintf("  Mean len:   %.0f\n", mean_len_par))
cat(sprintf("  Length range: %d-%d\n", min_len_par, max_len_par))
cat(sprintf("  Time:       %.3fs\n", parallel_time))
if (parallel_time > 0) {
  cat(sprintf("  Speedup:    %.1fx\n", serial_time / parallel_time))
}

# --- Streaming GC analysis ---
cat("\n-- Streaming GC Analysis --\n")
t0 <- proc.time()
con <- file("data/sample.fastq", "r")
stream_gc_sum <- 0.0
stream_len_sum <- 0
stream_high <- 0
stream_count <- 0
stream_min <- .Machine$integer.max
stream_max <- 0

while (TRUE) {
  header <- readLines(con, n = 1)
  if (length(header) == 0) break
  seq_line <- readLines(con, n = 1)
  plus_line <- readLines(con, n = 1)
  qual_line <- readLines(con, n = 1)

  gc <- gc_content(seq_line)
  l <- nchar(seq_line)
  stream_gc_sum <- stream_gc_sum + gc
  stream_len_sum <- stream_len_sum + l
  if (gc > 0.5) stream_high <- stream_high + 1
  if (l < stream_min) stream_min <- l
  if (l > stream_max) stream_max <- l
  stream_count <- stream_count + 1
}
close(con)
stream_time <- (proc.time() - t0)["elapsed"]

cat(sprintf("  Mean GC:    %.1f%%\n", stream_gc_sum / stream_count * 100))
cat(sprintf("  High GC:    %d reads\n", stream_high))
cat(sprintf("  Mean len:   %.0f\n", stream_len_sum / stream_count))
cat(sprintf("  Length range: %d-%d\n", stream_min, stream_max))
cat(sprintf("  Time:       %.3fs\n", stream_time))
cat("  Memory:     constant (streaming)\n")

# --- K-mer counting on FASTA ---
cat("\n-- K-mer Counting (FASTA, k=6) --\n")
fasta_seqs <- read_fasta_simple("data/sequences.fasta")
cat(sprintf("Loaded %d sequences\n", length(fasta_seqs)))

cat("\nSerial k-mer counting:\n")
t0 <- proc.time()
kmer_counts_serial <- sapply(fasta_seqs, count_kmers, k = 6, USE.NAMES = FALSE)
kmer_serial_time <- (proc.time() - t0)["elapsed"]
cat(sprintf("  Time: %.3fs\n", kmer_serial_time))
cat(sprintf("  Total k-mers: %d\n", sum(kmer_counts_serial)))

cat("\nParallel k-mer counting:\n")
t0 <- proc.time()
cl <- makeCluster(n_cores)
clusterExport(cl, "count_kmers")
kmer_counts_par <- parSapply(cl, fasta_seqs, count_kmers, k = 6)
stopCluster(cl)
kmer_par_time <- (proc.time() - t0)["elapsed"]
cat(sprintf("  Time: %.3fs\n", kmer_par_time))
cat(sprintf("  Total k-mers: %d\n", sum(kmer_counts_par)))
if (kmer_par_time > 0) {
  cat(sprintf("  Speedup: %.1fx\n", kmer_serial_time / kmer_par_time))
}

# --- Summary ---
cat("\n", strrep("=", 60), "\n")
cat("Summary\n")
cat(strrep("=", 60), "\n")
cat(sprintf("%-15s %10s %12s\n", "Approach", "GC Time", "K-mer Time"))
cat(strrep("-", 40), "\n")
cat(sprintf("%-15s %9.3fs %11.3fs\n", "Serial", serial_time, kmer_serial_time))
cat(sprintf("%-15s %9.3fs %11.3fs\n", "Parallel", parallel_time, kmer_par_time))
cat(sprintf("%-15s %9.3fs %12s\n", "Streaming", stream_time, "N/A"))
