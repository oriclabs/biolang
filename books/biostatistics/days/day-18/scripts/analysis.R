# Day 18: Experimental Design and Power Analysis
library(pwr)

data <- read.csv("power_simulation.csv")

# Empirical power
cat("=== Empirical Power (from simulation) ===\n")
for (n in sort(unique(data$sample_size))) {
  power <- mean(data$significant[data$sample_size == n])
  cat(sprintf("  n=%3d: power=%.3f\n", n, power))
}

# Analytical power
cat("\n=== Analytical Power (pwr) ===\n")
for (n in c(5, 10, 20, 50)) {
  result <- pwr.t.test(n = n, d = 0.8, sig.level = 0.05, type = "two.sample")
  cat(sprintf("  n=%3d: power=%.3f\n", n, result$power))
}

# Required sample size
result <- pwr.t.test(d = 0.8, power = 0.8, sig.level = 0.05, type = "two.sample")
cat(sprintf("\nRequired n per group for 80%% power: %d\n", ceiling(result$n)))

# Power curve
d_values <- seq(0.2, 1.5, by = 0.1)
powers <- sapply(d_values, function(d) {
  pwr.t.test(n = 20, d = d, sig.level = 0.05)$power
})
plot(d_values, powers, type = "b", xlab = "Effect Size (d)", ylab = "Power",
     main = "Power Curve (n=20)")
abline(h = 0.8, lty = 2, col = "red")
