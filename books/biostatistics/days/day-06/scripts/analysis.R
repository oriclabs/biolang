# Day 6: Confidence Intervals

ic50 <- read.csv("ic50.csv")$ic50_uM
vaccine <- read.csv("vaccine_trial.csv")$responded

# IC50: t-based CI
cat("=== IC50 (t-interval) ===\n")
tt <- t.test(ic50)
cat(sprintf("Mean: %.2f uM\n", tt$estimate))
cat(sprintf("95%% CI: (%.2f, %.2f)\n", tt$conf.int[1], tt$conf.int[2]))

# Vaccine trial: proportion CI
cat("\n=== Vaccine Efficacy ===\n")
x <- sum(vaccine)
n <- length(vaccine)
pt <- prop.test(x, n, correct = FALSE)
cat(sprintf("p-hat: %.3f\n", x / n))
cat(sprintf("95%% CI: (%.3f, %.3f)\n", pt$conf.int[1], pt$conf.int[2]))

# Bootstrap CI for IC50
set.seed(42)
boot_means <- replicate(10000, mean(sample(ic50, replace = TRUE)))
cat(sprintf("\nBootstrap 95%% CI: (%.2f, %.2f)\n",
            quantile(boot_means, 0.025), quantile(boot_means, 0.975)))
