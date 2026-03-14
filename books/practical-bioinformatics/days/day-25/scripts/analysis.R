#!/usr/bin/env Rscript
# Day 25: Error Handling in Production - R equivalent

library(jsonlite)
suppressPackageStartupMessages(library(futile.logger))

flog.appender(appender.tee("data/output/pipeline.log"))
flog.threshold(INFO)

classify_error <- function(err_msg) {
  msg <- tolower(as.character(err_msg))
  if (grepl("not found|no such file", msg)) return("missing")
  if (grepl("permission", msg)) return("access")
  if (grepl("timeout", msg)) return("transient")
  if (grepl("parse|invalid", msg)) return("data_corrupt")
  if (grepl("disk|space|quota", msg)) return("resource")
  return("unknown")
}

validate_fastq_file <- function(path) {
  if (!file.exists(path)) {
    stop(paste("File not found:", path))
  }
  if (!grepl("\\.fastq$", path)) {
    stop(paste("Expected .fastq file, got:", path))
  }
  if (file.size(path) == 0) {
    stop(paste("File is empty:", path))
  }
  TRUE
}

parse_fastq <- function(path) {
  lines <- readLines(path, warn = FALSE)
  lines <- lines[nchar(trimws(lines)) > 0]

  if (length(lines) == 0) return(list())

  records <- list()
  i <- 1
  while (i + 3 <= length(lines)) {
    if (!startsWith(lines[i], "@")) {
      stop(paste("Expected @ at line", i, "got:", substr(lines[i], 1, 20)))
    }
    records[[length(records) + 1]] <- list(
      id = substring(lines[i], 2),
      sequence = lines[i + 1],
      quality = lines[i + 3]
    )
    i <- i + 4
  }

  records
}

gc_content <- function(seq) {
  if (nchar(seq) == 0) return(0.0)
  chars <- strsplit(toupper(seq), "")[[1]]
  sum(chars %in% c("G", "C")) / length(chars)
}

quality_filter <- function(records, min_qual = 20) {
  Filter(function(rec) {
    quals <- utf8ToInt(rec$quality) - 33
    length(quals) > 0 && mean(quals) >= min_qual
  }, records)
}

retry_fn <- function(f, max_attempts = 3, base_delay = 1) {
  last_error <- NULL
  for (i in seq_len(max_attempts)) {
    result <- tryCatch(f(), error = function(e) {
      last_error <<- e
      NULL
    })
    if (!is.null(result)) return(result)
    if (i < max_attempts) {
      Sys.sleep(base_delay * 2^(i - 1))
    }
  }
  stop(paste("Failed after", max_attempts, "attempts:",
             conditionMessage(last_error)))
}

error_log <- list(entries = list())

log_error_entry <- function(source, severity, message) {
  entry <- list(
    timestamp = format(Sys.time(), "%Y-%m-%d %H:%M:%S"),
    source = source,
    severity = severity,
    message = message
  )
  error_log$entries[[length(error_log$entries) + 1]] <<- entry
  if (severity == "ERROR") {
    flog.error("%s: %s", source, message)
  } else if (severity == "WARN") {
    flog.warn("%s: %s", source, message)
  } else {
    flog.info("%s: %s", source, message)
  }
}

save_error_log <- function(path) {
  if (length(error_log$entries) > 0) {
    df <- do.call(rbind, lapply(error_log$entries, as.data.frame,
                                stringsAsFactors = FALSE))
    write.csv(df, path, row.names = FALSE)
  } else {
    writeLines("timestamp,source,severity,message", path)
  }
}

process_file <- function(path) {
  filename <- basename(path)

  valid <- tryCatch({
    validate_fastq_file(path)
    TRUE
  }, error = function(e) {
    log_error_entry(filename, "ERROR", conditionMessage(e))
    FALSE
  })
  if (!valid) return(NULL)

  records <- tryCatch(
    parse_fastq(path),
    error = function(e) {
      log_error_entry(filename, "ERROR",
                      paste("FASTQ parse failed:", conditionMessage(e)))
      NULL
    }
  )
  if (is.null(records)) return(NULL)

  if (length(records) == 0) {
    log_error_entry(filename, "WARN", "No records found, skipping")
    return(NULL)
  }

  valid_recs <- Filter(function(r) {
    nchar(r$sequence) == nchar(r$quality)
  }, records)

  if (length(valid_recs) < length(records)) {
    dropped <- length(records) - length(valid_recs)
    log_error_entry(filename, "WARN",
                    paste(dropped, "records had seq/qual length mismatch"))
  }

  filtered <- tryCatch(
    quality_filter(valid_recs, min_qual = 20),
    error = function(e) valid_recs
  )

  if (length(filtered) == 0) {
    log_error_entry(filename, "WARN",
                    "All records filtered out by quality threshold")
  }

  gc_vals <- if (length(filtered) > 0) {
    sapply(filtered, function(r) gc_content(r$sequence))
  } else {
    numeric(0)
  }
  mean_gc <- if (length(gc_vals) > 0) mean(gc_vals) else 0.0

  data.frame(
    file = filename,
    total_records = length(records),
    valid_records = length(valid_recs),
    passed_qc = length(filtered),
    pct_passed = if (length(valid_recs) > 0) {
      as.integer(length(filtered) * 100 / length(valid_recs))
    } else { 0L },
    mean_gc = round(mean_gc, 4),
    stringsAsFactors = FALSE
  )
}

summarize_run <- function(total, successes, failures) {
  success_rate <- if (total > 0) as.integer(successes * 100 / total) else 0L
  status <- if (failures == 0) {
    "COMPLETE"
  } else if (success_rate > 90) {
    "PARTIAL_SUCCESS"
  } else {
    "FAILED"
  }

  list(
    total_samples = total,
    succeeded = successes,
    failed = failures,
    success_rate_pct = success_rate,
    error_count = length(error_log$entries),
    status = status
  )
}

main <- function() {
  input_dir <- "data/fastq"
  output_dir <- "data/output"
  dir.create(output_dir, showWarnings = FALSE, recursive = TRUE)

  files <- tryCatch({
    ff <- list.files(input_dir, pattern = "\\.fastq$", full.names = FALSE)
    sort(ff)
  }, error = function(e) {
    log_error_entry(input_dir, "FATAL",
                    paste("Cannot list directory:", conditionMessage(e)))
    save_error_log(file.path(output_dir, "error_log.csv"))
    stop(e)
  })

  if (length(files) == 0) {
    stop(paste("No FASTQ files found in", input_dir))
  }

  results <- list()
  for (filename in files) {
    path <- file.path(input_dir, filename)
    result <- process_file(path)
    if (!is.null(result)) {
      results[[length(results) + 1]] <- result
    }
  }

  n_success <- length(results)
  n_fail <- length(files) - n_success
  summary_data <- summarize_run(length(files), n_success, n_fail)

  if (length(results) > 0) {
    results_df <- do.call(rbind, results)
    write.csv(results_df, file.path(output_dir, "qc_results.csv"),
              row.names = FALSE)
  }

  save_error_log(file.path(output_dir, "error_log.csv"))
  write_json(summary_data, file.path(output_dir, "summary.json"),
             pretty = TRUE, auto_unbox = TRUE)

  flog.info("Pipeline %s: %d/%d samples processed",
            summary_data$status, n_success, length(files))
}

main()
