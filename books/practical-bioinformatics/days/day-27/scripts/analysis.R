# --- seq_utils module equivalent ---

validate_dna <- function(seq) {
  upper_seq <- toupper(seq)
  chars <- strsplit(upper_seq, "")[[1]]
  valid <- c("A", "C", "G", "T", "N")
  invalid <- chars[!chars %in% valid]
  if (length(invalid) > 0) {
    stop(paste("Invalid DNA characters:", paste(invalid, collapse = ", ")))
  }
  upper_seq
}

compute_gc <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  gc_count <- sum(chars %in% c("G", "C"))
  gc_count / length(chars)
}

classify_gc <- function(seq) {
  clean <- validate_dna(seq)
  gc <- compute_gc(clean)
  if (gc > 0.6) {
    list(class = "high", gc = gc, label = "GC-rich")
  } else if (gc < 0.4) {
    list(class = "low", gc = gc, label = "AT-rich")
  } else {
    list(class = "moderate", gc = gc, label = "balanced")
  }
}

find_all_motifs <- function(seq, motif) {
  clean <- validate_dna(seq)
  motif_upper <- toupper(motif)
  positions <- c()
  start <- 1
  while (TRUE) {
    pos <- regexpr(motif_upper, substr(clean, start, nchar(clean)), fixed = TRUE)
    if (pos == -1) break
    actual_pos <- start + pos - 1
    positions <- c(positions, actual_pos)
    start <- actual_pos + 1
  }
  list(motif = motif_upper, count = length(positions), positions = positions)
}

batch_gc <- function(sequences) {
  lapply(sequences, function(seq) {
    result <- classify_gc(seq$sequence)
    list(
      id = seq$id,
      length = nchar(seq$sequence),
      gc = result$gc,
      class = result$class,
      label = result$label
    )
  })
}

sequence_summary <- function(sequences) {
  classified <- batch_gc(sequences)
  high <- sum(sapply(classified, function(s) s$class == "high"))
  low <- sum(sapply(classified, function(s) s$class == "low"))
  moderate <- sum(sapply(classified, function(s) s$class == "moderate"))
  gc_values <- sapply(classified, function(s) s$gc)
  list(
    total = length(sequences),
    high_gc = high,
    low_gc = low,
    moderate_gc = moderate,
    mean_gc = mean(gc_values),
    stdev_gc = sd(gc_values)
  )
}

# --- qc module equivalent ---

length_stats <- function(sequences) {
  lengths <- sapply(sequences, function(s) nchar(s$sequence))
  list(
    count = length(lengths),
    min_len = min(lengths),
    max_len = max(lengths),
    mean_len = mean(lengths),
    median_len = median(lengths)
  )
}

gc_distribution <- function(sequences) {
  gc_values <- sapply(sequences, function(s) compute_gc(s$sequence))
  list(
    mean_gc = mean(gc_values),
    min_gc = min(gc_values),
    max_gc = max(gc_values),
    stdev_gc = sd(gc_values)
  )
}

flag_outliers <- function(sequences, min_len, max_len, min_gc, max_gc) {
  lapply(sequences, function(s) {
    gc <- compute_gc(s$sequence)
    slen <- nchar(s$sequence)
    flags <- c()
    if (slen < min_len) flags <- c(flags, "too_short")
    if (slen > max_len) flags <- c(flags, "too_long")
    if (gc < min_gc) flags <- c(flags, "low_gc")
    if (gc > max_gc) flags <- c(flags, "high_gc")
    list(id = s$id, length = slen, gc = gc, flags = flags, pass = length(flags) == 0)
  })
}

qc_summary <- function(sequences) {
  lstats <- length_stats(sequences)
  gc_dist <- gc_distribution(sequences)
  flagged <- flag_outliers(sequences, 50, 10000, 0.2, 0.8)
  passing <- sum(sapply(flagged, function(f) f$pass))
  failing <- sum(sapply(flagged, function(f) !f$pass))
  list(
    total = lstats$count,
    passing = passing,
    failing = failing,
    pass_rate = passing / lstats$count,
    length = lstats,
    gc = gc_dist
  )
}

format_qc_report <- function(summary) {
  c(
    sprintf("Sequences: %d", summary$total),
    sprintf("Passing QC: %d", summary$passing),
    sprintf("Failing QC: %d", summary$failing),
    sprintf("Length range: %d-%d", summary$length$min_len, summary$length$max_len),
    sprintf("Mean length: %.1f", summary$length$mean_len),
    sprintf("Mean GC: %.4f", summary$gc$mean_gc),
    sprintf("GC stdev: %.4f", summary$gc$stdev_gc)
  )
}

# --- I/O ---

read_fasta <- function(path) {
  lines <- readLines(path)
  sequences <- list()
  current_id <- NULL
  current_seq <- ""
  for (line in lines) {
    if (startsWith(line, ">")) {
      if (!is.null(current_id)) {
        sequences[[length(sequences) + 1]] <- list(id = current_id, sequence = current_seq)
      }
      current_id <- sub("^>", "", line)
      current_seq <- ""
    } else {
      current_seq <- paste0(current_seq, line)
    }
  }
  if (!is.null(current_id)) {
    sequences[[length(sequences) + 1]] <- list(id = current_id, sequence = current_seq)
  }
  sequences
}

# --- Main pipeline ---

main <- function() {
  sequences <- read_fasta("data/sequences.fasta")
  qc_reads <- read_fasta("data/qc_reads.fasta")

  summary <- sequence_summary(sequences)
  classified <- batch_gc(sequences)

  tata_hits <- lapply(sequences, function(s) find_all_motifs(s$sequence, "TATAAA"))
  ecori_hits <- lapply(sequences, function(s) find_all_motifs(s$sequence, "GAATTC"))

  qc_result <- qc_summary(qc_reads)
  qc_report <- format_qc_report(qc_result)

  flagged <- flag_outliers(qc_reads, 50, 10000, 0.2, 0.8)
  passing_ids <- sapply(flagged[sapply(flagged, function(f) f$pass)], function(f) f$id)
  failing <- flagged[sapply(flagged, function(f) !f$pass)]

  passing_seqs <- sequences[sapply(qc_reads, function(s) s$id %in% passing_ids)]
  passing_classified <- batch_gc(passing_seqs)

  dir.create("data/output", showWarnings = FALSE, recursive = TRUE)

  out <- file("data/output/report.txt", "w")
  writeLines("# Module-Based Analysis Report", out)
  writeLines("", out)
  writeLines("## Sequence Summary (seq_utils module)", out)
  writeLines(sprintf("Total sequences: %d", summary$total), out)
  writeLines(sprintf("High GC: %d", summary$high_gc), out)
  writeLines(sprintf("Low GC: %d", summary$low_gc), out)
  writeLines(sprintf("Moderate GC: %d", summary$moderate_gc), out)
  writeLines(sprintf("Mean GC: %.4f", summary$mean_gc), out)
  writeLines(sprintf("GC stdev: %.4f", summary$stdev_gc), out)
  writeLines("", out)
  writeLines("## GC Classification", out)
  for (c in classified) {
    writeLines(sprintf("  %s: %s (GC=%.4f)", c$id, c$class, c$gc), out)
  }
  writeLines("", out)
  writeLines("## Motif Search", out)
  writeLines(sprintf("TATAAA hits: %d", sum(sapply(tata_hits, function(h) h$count))), out)
  writeLines(sprintf("GAATTC (EcoRI) hits: %d", sum(sapply(ecori_hits, function(h) h$count))), out)
  writeLines("", out)
  writeLines("## QC Report (qc module)", out)
  for (line in qc_report) {
    writeLines(line, out)
  }
  writeLines("", out)
  writeLines("## Failing Sequences", out)
  for (f in failing) {
    writeLines(sprintf("  %s: flags=%s", f$id, paste(f$flags, collapse = ", ")), out)
  }
  writeLines("", out)
  writeLines("## Passing Sequences - GC Classification", out)
  for (c in passing_classified) {
    writeLines(sprintf("  %s: %s (GC=%.4f)", c$id, c$class, c$gc), out)
  }
  close(out)

  classified_df <- do.call(rbind, lapply(classified, function(c) {
    data.frame(id = c$id, length = c$length, gc = c$gc, class = c$class, label = c$label,
               stringsAsFactors = FALSE)
  }))
  write.csv(classified_df, "data/output/gc_classification.csv", row.names = FALSE)

  cat("Report written to data/output/report.txt\n")
}

main()
