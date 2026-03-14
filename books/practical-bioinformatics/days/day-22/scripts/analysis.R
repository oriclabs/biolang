# Day 22: Reproducible QC Pipeline — R equivalent
#
# Reads config from config.json, processes FASTQ files with quality/length
# filtering, computes summary statistics, writes CSV output and a provenance
# JSON log with SHA-256 checksums for all inputs and outputs.

library(jsonlite)
library(digest)
library(logging)

basicConfig(level = "INFO")

load_config <- function(path) {
  fromJSON(path)
}

validate_config <- function(config) {
  errors <- character(0)
  required <- c("pipeline_name", "version", "input_files",
                 "output_dir", "min_quality", "min_length")
  for (key in required) {
    if (is.null(config[[key]])) {
      errors <- c(errors, paste0("Missing required field: ", key))
    }
  }
  for (f in config$input_files) {
    if (!file.exists(f)) {
      errors <- c(errors, paste0("Missing input file: ", f))
    }
  }
  mq <- config$min_quality
  if (!is.null(mq) && (mq < 0 || mq > 40)) {
    errors <- c(errors, sprintf("min_quality must be 0-40, got %d", mq))
  }
  ml <- config$min_length
  if (!is.null(ml) && ml < 1) {
    errors <- c(errors, sprintf("min_length must be >= 1, got %d", ml))
  }
  errors
}

sha256_file <- function(path) {
  digest(file = path, algo = "sha256")
}

checksum_files <- function(paths) {
  lapply(paths, function(p) {
    list(file = p, sha256 = sha256_file(p))
  })
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

process_sample <- function(file_path, config) {
  t0 <- proc.time()["elapsed"]
  reads <- parse_fastq(file_path)
  total_count <- length(reads)

  min_qual <- config$min_quality
  min_len <- config$min_length

  filtered <- Filter(function(r) mean(r$qual) >= min_qual, reads)
  length_filtered <- Filter(function(r) nchar(r$seq) >= min_len, filtered)
  pass_count <- length(length_filtered)

  gc_values <- sapply(length_filtered, function(r) gc_content(r$seq))
  lengths <- sapply(length_filtered, function(r) nchar(r$seq))
  qualities <- sapply(length_filtered, function(r) mean(r$qual))

  elapsed <- as.numeric(proc.time()["elapsed"] - t0)

  list(
    file = file_path,
    total_reads = total_count,
    passed_reads = pass_count,
    pass_rate = if (total_count > 0) pass_count / total_count else 0,
    gc_mean = if (length(gc_values) > 0) mean(gc_values) else 0,
    gc_stdev = if (length(gc_values) > 1) sd(gc_values) else 0,
    length_mean = if (length(lengths) > 0) mean(lengths) else 0,
    length_min = if (length(lengths) > 0) min(lengths) else 0,
    length_max = if (length(lengths) > 0) max(lengths) else 0,
    quality_mean = if (length(qualities) > 0) mean(qualities) else 0,
    elapsed_seconds = elapsed
  )
}

create_provenance <- function(config) {
  list(
    pipeline = config$pipeline_name,
    version = config$version,
    started_at = format(Sys.time(), "%Y-%m-%d %H:%M:%S"),
    parameters = config,
    input_checksums = list(),
    steps = list(),
    output_checksums = list(),
    finished_at = NULL,
    status = "running"
  )
}

log_step <- function(prov, step_name, details) {
  step <- list(
    name = step_name,
    timestamp = format(Sys.time(), "%Y-%m-%d %H:%M:%S"),
    details = details
  )
  prov$steps <- c(prov$steps, list(step))
  prov
}

finish_provenance <- function(prov, status) {
  prov$finished_at <- format(Sys.time(), "%Y-%m-%d %H:%M:%S")
  prov$status <- status
  prov
}

save_provenance <- function(prov, log_dir) {
  ts <- format(Sys.time(), "%Y%m%d_%H%M%S")
  filename <- file.path(log_dir, paste0("provenance_", ts, ".json"))
  writeLines(toJSON(prov, auto_unbox = TRUE, pretty = TRUE), filename)
  filename
}

main <- function() {
  config <- load_config("config.json")

  errors <- validate_config(config)
  if (length(errors) > 0) {
    for (e in errors) logwarn(e)
    stop("Configuration invalid")
  }

  dir.create(config$output_dir, showWarnings = FALSE, recursive = TRUE)
  log_dir <- if (!is.null(config$log_dir)) config$log_dir else "logs"
  dir.create(log_dir, showWarnings = FALSE, recursive = TRUE)

  prov <- create_provenance(config)
  loginfo("Pipeline %s v%s started", config$pipeline_name, config$version)

  input_checksums <- checksum_files(config$input_files)
  prov$input_checksums <- input_checksums
  prov <- log_step(prov, "checksum_inputs", list(file_count = length(input_checksums)))
  for (c in input_checksums) {
    loginfo("  Input: %s -> %s", c$file, c$sha256)
  }

  results <- list()
  for (f in config$input_files) {
    loginfo("Processing: %s", f)
    result <- process_sample(f, config)
    loginfo("  %d/%d reads passed (%d%%)",
            result$passed_reads, result$total_reads,
            as.integer(result$pass_rate * 100))
    results <- c(results, list(result))
  }

  prov <- log_step(prov, "process_samples", list(
    sample_count = length(results),
    total_reads = sum(sapply(results, `[[`, "total_reads")),
    total_passed = sum(sapply(results, `[[`, "passed_reads"))
  ))

  output_path <- file.path(config$output_dir, "qc_summary.csv")
  summary_df <- data.frame(
    file = sapply(results, `[[`, "file"),
    total_reads = sapply(results, `[[`, "total_reads"),
    passed_reads = sapply(results, `[[`, "passed_reads"),
    pass_rate = sapply(results, `[[`, "pass_rate"),
    gc_mean = sapply(results, `[[`, "gc_mean"),
    length_mean = sapply(results, `[[`, "length_mean"),
    quality_mean = sapply(results, `[[`, "quality_mean"),
    stringsAsFactors = FALSE
  )
  write.csv(summary_df, output_path, row.names = FALSE)
  loginfo("Summary written to: %s", output_path)
  prov <- log_step(prov, "write_results", list(output_file = output_path))

  output_checksums <- checksum_files(c(output_path))
  prov$output_checksums <- output_checksums
  prov <- finish_provenance(prov, "success")

  prov_file <- save_provenance(prov, log_dir)
  loginfo("Provenance saved to: %s", prov_file)
  loginfo("Pipeline completed successfully.")
}

main()
