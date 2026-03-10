# Benchmark 5: CSV Wrangling
suppressPackageStartupMessages(library(dplyr))

# Ensure working directory is the benchmarks root
# When run via run_all.ps1, cwd is already correct; this is a safety net
script_dir <- tryCatch(dirname(sys.frame(1)$ofile), error = function(e) NULL)
if (!is.null(script_dir) && nzchar(script_dir)) {
  setwd(file.path(script_dir, ".."))
}

tryCatch({

samples <- read.csv("data/samples.csv", stringsAsFactors = FALSE)
metadata <- read.csv("data/metadata.csv", stringsAsFactors = FALSE)

# Join
joined <- inner_join(samples, metadata, by = "sample_id")

# Group by cohort, summarize
summary_df <- joined %>%
  group_by(cohort) %>%
  summarize(
    count = n(),
    mean_depth = mean(depth),
    mean_quality = mean(quality),
    total_reads = sum(read_count),
    .groups = "drop"
  ) %>%
  arrange(desc(mean_depth))

cat("Cohort Summary:\n")
for (i in seq_len(nrow(summary_df))) {
  row <- summary_df[i, ]
  cat(sprintf("  %s: n=%.0f, depth=%.1f, qual=%.1f, reads=%.0f\n",
              row$cohort, row$count, row$mean_depth, row$mean_quality, row$total_reads))
}

# High-quality filter
hq <- joined %>% filter(quality >= 30, depth >= 20)
cat(sprintf("\nHigh-quality samples: %.0f / %.0f\n", nrow(hq), nrow(joined)))

}, error = function(e) {
  cat(paste("ERROR:", conditionMessage(e), "\n"), file = stderr())
  quit(status = 1, save = "no")
})
