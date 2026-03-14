# Day 27: Reproducible Analysis
# Structured, seed-controlled, parameterized

# --- Configuration ---
config <- list(
  seed = 42,
  alpha = 0.05,
  n_bootstrap = 10000
)
set.seed(config$seed)

# --- Load Data ---
data <- read.csv("reproducible_data.csv")
trt <- data$value[data$group == "Treatment"]
ctrl <- data$value[data$group == "Control"]

# --- Analysis ---
tt <- t.test(trt, ctrl)
pooled_sd <- sqrt(((length(trt)-1)*sd(trt)^2 + (length(ctrl)-1)*sd(ctrl)^2) /
                  (length(trt)+length(ctrl)-2))
d <- (mean(trt) - mean(ctrl)) / pooled_sd

# --- Results ---
results <- list(
  timestamp = Sys.time(),
  seed = config$seed,
  n_treatment = length(trt),
  n_control = length(ctrl),
  mean_diff = round(mean(trt) - mean(ctrl), 4),
  t_statistic = round(tt$statistic, 4),
  p_value = round(tt$p.value, 4),
  cohens_d = round(d, 4),
  ci_lower = round(tt$conf.int[1], 4),
  ci_upper = round(tt$conf.int[2], 4)
)

cat("=== Reproducible Analysis Results ===\n")
for (name in names(results)) {
  cat(sprintf("  %s: %s\n", name, results[[name]]))
}

# Save
saveRDS(results, "results.rds")
cat("\nResults saved to results.rds\n")
cat(sprintf("Session: R %s\n", R.version.string))
