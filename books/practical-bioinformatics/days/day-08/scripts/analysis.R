#!/usr/bin/env Rscript
# Day 8: Processing Large Files — R streaming equivalent
# Uses connection-based reading and chunked processing
# Run: Rscript scripts/analysis.R
# Requires: Rscript r/install.R

cat("============================================================\n")
cat("Day 8: Processing Large Files — R Streaming\n")
cat("============================================================\n")

# ---- Helper functions ----

mean_phred <- function(qual_str) {
  scores <- as.integer(charToRaw(qual_str)) - 33L
  mean(scores)
}

gc_content <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  sum(chars %in% c("G", "C")) / length(chars)
}

# ---- Streaming FASTQ reader (connection-based) ----
# R doesn't have generators, so we use connection + read-4-lines pattern

read_fastq_chunk <- function(con, n = 100) {
  records <- list()
  for (i in seq_len(n)) {
    header <- readLines(con, n = 1)
    if (length(header) == 0) break
    seq <- readLines(con, n = 1)
    readLines(con, n = 1)  # + line
    qual <- readLines(con, n = 1)
    read_id <- sub("^@(\\S+).*", "\\1", header)
    records[[i]] <- list(id = read_id, seq = seq, qual = qual)
  }
  records
}

count_fastq_stream <- function(path) {
  con <- file(path, "r")
  on.exit(close(con))
  total <- 0L
  repeat {
    chunk <- read_fastq_chunk(con, 1000)
    if (length(chunk) == 0) break
    total <- total + length(chunk)
  }
  total
}

# ---- Streaming FASTA reader ----

stream_fasta <- function(path) {
  lines <- readLines(path)
  records <- list()
  name <- NULL
  seq_parts <- character()
  for (line in lines) {
    if (startsWith(line, ">")) {
      if (!is.null(name)) {
        records[[length(records) + 1]] <- list(id = name, seq = paste(seq_parts, collapse = ""))
      }
      name <- sub("^>(\\S+).*", "\\1", line)
      seq_parts <- character()
    } else {
      seq_parts <- c(seq_parts, line)
    }
  }
  if (!is.null(name)) {
    records[[length(records) + 1]] <- list(id = name, seq = paste(seq_parts, collapse = ""))
  }
  records
}

# ---- Streaming VCF reader ----

stream_vcf <- function(path) {
  lines <- readLines(path)
  records <- list()
  for (line in lines) {
    if (startsWith(line, "#")) next
    fields <- strsplit(line, "\t")[[1]]
    if (length(fields) >= 7) {
      records[[length(records) + 1]] <- list(
        chrom = fields[1],
        pos = as.integer(fields[2]),
        ref = fields[4],
        alt = fields[5],
        qual = fields[6],
        filter = fields[7]
      )
    }
  }
  records
}

# ---- Streaming BED reader ----

stream_bed <- function(path) {
  lines <- readLines(path)
  records <- list()
  for (line in lines) {
    fields <- strsplit(line, "\t")[[1]]
    if (length(fields) >= 3) {
      records[[length(records) + 1]] <- list(
        chrom = fields[1],
        start = as.integer(fields[2]),
        end = as.integer(fields[3]),
        name = if (length(fields) > 3) fields[4] else ""
      )
    }
  }
  records
}

# ============================================================
# Main analysis
# ============================================================

# ----------------------------------------------------------
# 1. Eager vs Streaming
# ----------------------------------------------------------
cat("\n--- 1. Eager vs Streaming ---\n")

# Eager: load all via connection
con <- file("data/reads.fastq", "r")
all_reads <- list()
repeat {
  chunk <- read_fastq_chunk(con, 1000)
  if (length(chunk) == 0) break
  all_reads <- c(all_reads, chunk)
}
close(con)
cat(sprintf("Eager: loaded %d reads (type: list)\n", length(all_reads)))

# Streaming: count without storing
stream_count <- count_fastq_stream("data/reads.fastq")
cat(sprintf("Stream type: connection\n"))
cat(sprintf("Stream count: %d\n", stream_count))

# ----------------------------------------------------------
# 2. Constant-Memory Patterns
# ----------------------------------------------------------
cat("\n--- 2. Constant-Memory Patterns ---\n")

# Count
total <- count_fastq_stream("data/reads.fastq")
cat(sprintf("Total reads: %d\n", total))

# Filter and count (chunked)
con <- file("data/reads.fastq", "r")
passed_q20 <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    if (mean_phred(r$qual) >= 20) passed_q20 <- passed_q20 + 1L
  }
}
close(con)
cat(sprintf("Passed Q20: %d\n", passed_q20))

# Reduce for mean GC (chunked)
con <- file("data/reads.fastq", "r")
gc_sum <- 0.0
gc_n <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    gc_sum <- gc_sum + gc_content(r$seq)
    gc_n <- gc_n + 1L
  }
}
close(con)
cat(sprintf("Mean GC: %.1f%%\n", gc_sum / gc_n * 100))

# Sample first 5
cat("\nFirst 5 reads:\n")
con <- file("data/reads.fastq", "r")
sample_reads <- read_fastq_chunk(con, 5)
close(con)
for (r in sample_reads) {
  cat(sprintf("  %s: %d bp, Q=%.1f\n", r$id, nchar(r$seq), mean_phred(r$qual)))
}

# ----------------------------------------------------------
# 3. Lazy Pipeline
# ----------------------------------------------------------
cat("\n--- 3. Lazy Pipeline ---\n")

con <- file("data/reads.fastq", "r")
hq_count <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    if (mean_phred(r$qual) >= 30) {
      hq_count <- hq_count + 1L
      if (hq_count >= 1000) break
    }
  }
  if (hq_count >= 1000) break
}
close(con)
cat(sprintf("High-quality reads (Q>=30): %d\n", hq_count))

# ----------------------------------------------------------
# 4. Chunked Processing
# ----------------------------------------------------------
cat("\n--- 4. Chunked Processing ---\n")

con <- file("data/reads.fastq", "r")
batch_num <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 100)
  if (length(chunk) == 0) break
  batch_num <- batch_num + 1L
  gc_vals <- sapply(chunk, function(r) gc_content(r$seq))
  cat(sprintf("  Batch %d: %d reads, mean GC: %.1f%%\n",
              batch_num, length(chunk), mean(gc_vals) * 100))
}
close(con)

# ----------------------------------------------------------
# 5. Streaming All Formats
# ----------------------------------------------------------
cat("\n--- 5. Streaming All Formats ---\n")

# FASTA: highest GC
fasta_recs <- stream_fasta("data/sequences.fasta")
gc_vals <- sapply(fasta_recs, function(s) gc_content(s$seq))
best_idx <- which.max(gc_vals)
cat(sprintf("FASTA highest GC: %s at %.1f%%\n",
            fasta_recs[[best_idx]]$id, gc_vals[best_idx] * 100))

# VCF: PASS counts by chrom
vcf_recs <- stream_vcf("data/variants.vcf")
pass_chroms <- sapply(
  Filter(function(v) v$filter == "PASS", vcf_recs),
  function(v) v$chrom
)
chr_counts <- table(pass_chroms)
counts_str <- paste(names(chr_counts), chr_counts, sep = ": ", collapse = ", ")
cat(sprintf("VCF PASS by chrom: {%s}\n", counts_str))

# BED: total covered
bed_recs <- stream_bed("data/regions.bed")
total_bp <- sum(sapply(bed_recs, function(r) r$end - r$start))
cat(sprintf("BED total covered: %d bp\n", total_bp))

# ----------------------------------------------------------
# 6. Tee Pattern (manual in R)
# ----------------------------------------------------------
cat("\n--- 6. Tee Pattern ---\n")

con <- file("data/reads.fastq", "r")
kept <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 1)
  if (length(chunk) == 0) break
  r <- chunk[[1]]
  cat(sprintf("  Checking: %s\n", r$id))
  if (mean_phred(r$qual) >= 35) {
    kept <- kept + 1L
    if (kept >= 3) break
  }
}
close(con)
cat(sprintf("Kept %d reads with Q>=35\n", kept))

# ----------------------------------------------------------
# 7. QC Report
# ----------------------------------------------------------
cat("\n--- 7. Streaming QC Report ---\n")

con <- file("data/reads.fastq", "r")
total_reads <- 0L
total_bases <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    total_reads <- total_reads + 1L
    total_bases <- total_bases + nchar(r$seq)
  }
}
close(con)
cat(sprintf("Total reads: %d\n", total_reads))
cat(sprintf("Total bases: %d\n", total_bases))

con <- file("data/reads.fastq", "r")
quality_bins <- c(excellent = 0L, good = 0L, poor = 0L)
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    q <- mean_phred(r$qual)
    if (q >= 30) quality_bins["excellent"] <- quality_bins["excellent"] + 1L
    else if (q >= 20) quality_bins["good"] <- quality_bins["good"] + 1L
    else quality_bins["poor"] <- quality_bins["poor"] + 1L
  }
}
close(con)
cat("Quality distribution:\n")
for (cat_name in names(quality_bins)) {
  cat(sprintf("  %s: %d\n", cat_name, quality_bins[cat_name]))
}

con <- file("data/reads.fastq", "r")
all_lengths <- integer()
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  all_lengths <- c(all_lengths, sapply(chunk, function(r) nchar(r$seq)))
}
close(con)
cat(sprintf("Length mean: %.1f\n", mean(all_lengths)))
cat(sprintf("Length min: %d, max: %d\n", min(all_lengths), max(all_lengths)))

# ----------------------------------------------------------
# 8. Write Filtered Output
# ----------------------------------------------------------
cat("\n--- 8. Write Filtered Output ---\n")

dir.create("results", showWarnings = FALSE)
con <- file("data/reads.fastq", "r")
out_lines <- character()
filtered_count <- 0L
repeat {
  chunk <- read_fastq_chunk(con, 500)
  if (length(chunk) == 0) break
  for (r in chunk) {
    if (nchar(r$seq) >= 100 && mean_phred(r$qual) >= 25) {
      out_lines <- c(out_lines, sprintf("@%s", r$id), r$seq, "+", r$qual)
      filtered_count <- filtered_count + 1L
    }
  }
}
close(con)
writeLines(out_lines, "results/filtered.fastq")
cat(sprintf("Wrote %d filtered reads to results/filtered.fastq\n", filtered_count))

cat("\n")
cat("============================================================\n")
cat("Day 8 complete! You can now process files of any size.\n")
cat("============================================================\n")
