# Day 23: Resampling Methods

data <- read.csv("small_sample.csv")
trt <- data$value[data$group == "Treatment"]
ctrl <- data$value[data$group == "Control"]
obs_diff <- mean(trt) - mean(ctrl)

# Bootstrap CI
set.seed(42)
n_boot <- 10000
boot_diffs <- replicate(n_boot, {
  b_trt <- sample(trt, replace = TRUE)
  b_ctrl <- sample(ctrl, replace = TRUE)
  mean(b_trt) - mean(b_ctrl)
})

cat("=== Bootstrap (10,000 resamples) ===\n")
cat(sprintf("Observed diff: %.2f\n", obs_diff))
cat(sprintf("Bootstrap mean: %.2f\n", mean(boot_diffs)))
cat(sprintf("95%% CI: (%.2f, %.2f)\n",
            quantile(boot_diffs, 0.025), quantile(boot_diffs, 0.975)))

# Permutation test
combined <- c(trt, ctrl)
n_trt <- length(trt)
perm_diffs <- replicate(n_boot, {
  perm <- sample(combined)
  mean(perm[1:n_trt]) - mean(perm[(n_trt+1):length(combined)])
})

p_perm <- mean(abs(perm_diffs) >= abs(obs_diff))
cat(sprintf("\n=== Permutation Test ===\nP-value: %.4f\n", p_perm))

# Comparison with parametric
cat(sprintf("\nt-test p-value: %.4f\n", t.test(trt, ctrl)$p.value))
