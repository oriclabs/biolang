# Day 23: Batch Processing and Automation — R equivalent
#
# Parses a sample sheet, processes all samples with quality filtering,
# tracks errors per sample, aggregates results into per-sample and
# per-group summaries, flags outliers, and writes a batch report.

library(jsonlite)
library(parallel)

load_config <- function(path) {
  fromJSON(path)
}

parse_sample_sheet <- function(path) {
  df <- read.csv(path, stringsAsFactors = FALSE)
  lapply(seq_len(nrow(df)), function(i) {
    list(
      id = df$sample_id[i],
      fastq = df$fastq_file[i],
      tissue = df$tissue[i],
      group = df$group[i]
    )
  })
}

validate_sample_sheet <- function(samples) {
  files <- sapply(samples, `[[`, "fastq")
  files[!file.exists(files)]
}

parse_fastq <- function(path) {
  lines <- readLines(path, warn = FALSE)
  n_reads <- length(lines) %/% 4
  reads <- vector("list", n_reads)
  for (i in seq_len(n_reads)) {
    offset <- (i - 1) * 4
    header <- lines[offset + 1]
    seq <- lines[offset + 2]
    qual_str <- lines[offset + 4]
    quals <- as.integer(charToRaw(qual_str)) - 33L
    reads[[i]] <- list(
      id = substring(header, 2),
      seq = seq,
      qual = quals
    )
  }
  reads
}

gc_content <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  gc <- sum(chars %in% c("G", "C"))
  gc / length(chars)
}

process_sample <- function(sample, config) {
  t0 <- proc.time()["elapsed"]
  tryCatch({
    reads <- parse_fastq(sample$fastq)
    total <- length(reads)

    min_qual <- config$min_quality
    min_len <- config$min_length

    filtered <- Filter(function(r) mean(r$qual) >= min_qual, reads)
    passed <- Filter(function(r) nchar(r$seq) >= min_len, filtered)
    pass_count <- length(passed)

    gc_values <- sapply(passed, function(r) gc_content(r$seq))
    lengths <- sapply(passed, function(r) nchar(r$seq))

    elapsed <- as.numeric(proc.time()["elapsed"] - t0)

    list(
      sample_id = sample$id,
      tissue = sample$tissue,
      group = sample$group,
      total_reads = total,
      passed_reads = pass_count,
      pass_rate = if (total > 0) pass_count / total else 0,
      gc_mean = if (length(gc_values) > 0) mean(gc_values) else 0,
      gc_stdev = if (length(gc_values) > 1) sd(gc_values) else 0,
      length_mean = if (length(lengths) > 0) mean(lengths) else 0,
      length_min = if (length(lengths) > 0) min(lengths) else 0,
      length_max = if (length(lengths) > 0) max(lengths) else 0,
      elapsed = elapsed,
      status = "ok",
      error_msg = NA_character_
    )
  }, error = function(e) {
    list(
      sample_id = sample$id,
      tissue = sample$tissue,
      group = sample$group,
      total_reads = 0, passed_reads = 0, pass_rate = 0,
      gc_mean = 0, gc_stdev = 0,
      length_mean = 0, length_min = 0, length_max = 0,
      elapsed = as.numeric(proc.time()["elapsed"] - t0),
      status = "error",
      error_msg = conditionMessage(e)
    )
  })
}

flag_outliers <- function(results, field) {
  ok_results <- Filter(function(r) r$status == "ok", results)
  values <- sapply(ok_results, `[[`, field)
  if (length(values) < 3) return(character(0))
  m <- mean(values)
  s <- sd(values)
  lower <- m - 2 * s
  upper <- m + 2 * s
  flagged <- Filter(function(r) {
    v <- r[[field]]
    v < lower || v > upper
  }, ok_results)
  sapply(flagged, `[[`, "sample_id")
}

summarize_by_group <- function(results) {
  ok_results <- Filter(function(r) r$status == "ok", results)
  groups <- unique(sapply(ok_results, `[[`, "group"))
  lapply(sort(groups), function(g) {
    gr <- Filter(function(r) r$group == g, ok_results)
    list(
      group = g,
      n_samples = length(gr),
      mean_pass_rate = mean(sapply(gr, `[[`, "pass_rate")),
      mean_gc = mean(sapply(gr, `[[`, "gc_mean")),
      mean_reads = mean(sapply(gr, `[[`, "total_reads"))
    )
  })
}

main <- function() {
  config <- load_config("config.json")
  samples <- parse_sample_sheet(config$sample_sheet)

  missing <- validate_sample_sheet(samples)
  if (length(missing) > 0) {
    stop(paste("Missing input files:", paste(missing, collapse = ", ")))
  }

  dir.create(config$output_dir, showWarnings = FALSE, recursive = TRUE)
  dir.create(config$log_dir, showWarnings = FALSE, recursive = TRUE)

  t0 <- proc.time()["elapsed"]

  n_cores <- min(detectCores(), length(samples))
  if (.Platform$OS.type == "windows") {
    cl <- makeCluster(n_cores)
    clusterExport(cl, c("parse_fastq", "gc_content", "process_sample", "config"),
                  envir = environment())
    all_results <- parLapply(cl, samples, function(s) process_sample(s, config))
    stopCluster(cl)
  } else {
    all_results <- mclapply(samples, function(s) process_sample(s, config),
                            mc.cores = n_cores)
  }

  total_time <- as.numeric(proc.time()["elapsed"] - t0)

  results <- Filter(function(r) r$status == "ok", all_results)
  error_list <- Filter(function(r) r$status == "error", all_results)
  errors <- lapply(error_list, function(r) {
    list(sample_id = r$sample_id, error = r$error_msg)
  })

  # Per-sample summary
  summary_df <- data.frame(
    sample_id = sapply(results, `[[`, "sample_id"),
    tissue = sapply(results, `[[`, "tissue"),
    group = sapply(results, `[[`, "group"),
    total_reads = sapply(results, `[[`, "total_reads"),
    passed_reads = sapply(results, `[[`, "passed_reads"),
    pass_rate = sapply(results, `[[`, "pass_rate"),
    gc_mean = sapply(results, `[[`, "gc_mean"),
    length_mean = sapply(results, `[[`, "length_mean"),
    stringsAsFactors = FALSE
  )
  summary_path <- file.path(config$output_dir, "batch_summary.csv")
  write.csv(summary_df, summary_path, row.names = FALSE)

  # Group summary
  group_stats <- summarize_by_group(all_results)
  group_df <- do.call(rbind, lapply(group_stats, as.data.frame))
  group_path <- file.path(config$output_dir, "group_summary.csv")
  write.csv(group_df, group_path, row.names = FALSE)

  # Outlier detection
  gc_outliers <- flag_outliers(all_results, "gc_mean")
  rate_outliers <- flag_outliers(all_results, "pass_rate")

  # Batch report
  report <- list(
    timestamp = format(Sys.time(), "%Y-%m-%d %H:%M:%S"),
    total_samples = length(samples),
    succeeded = length(results),
    failed = length(errors),
    total_time = total_time,
    gc_outliers = gc_outliers,
    rate_outliers = rate_outliers,
    errors = errors
  )
  report_path <- file.path(config$log_dir, "batch_report.json")
  writeLines(toJSON(report, auto_unbox = TRUE, pretty = TRUE), report_path)

  cat(sprintf("Batch complete: %d/%d succeeded in %.1fs\n",
              length(results), length(samples), total_time))
  cat(sprintf("Summary: %s\n", summary_path))
  cat(sprintf("Report:  %s\n", report_path))
}

main()
