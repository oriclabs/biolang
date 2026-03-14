# Day 5: Sampling and the Central Limit Theorem

pop <- read.csv("population.csv")$allele_freq
samples <- read.csv("sample_means.csv")

cat("=== Population ===\n")
cat(sprintf("N: %d, Mean: %.4f, SD: %.4f\n", length(pop), mean(pop), sd(pop)))

cat("\n=== Sample Means (n=20) ===\n")
m20 <- samples$mean_n20
cat(sprintf("Mean of means: %.4f\n", mean(m20)))
cat(sprintf("SD of means:   %.4f\n", sd(m20)))
cat(sprintf("Expected SE:   %.4f\n", sd(pop) / sqrt(20)))

cat("\n=== Sample Means (n=200) ===\n")
m200 <- samples$mean_n200
cat(sprintf("Mean of means: %.4f\n", mean(m200)))
cat(sprintf("SD of means:   %.4f\n", sd(m200)))
cat(sprintf("Expected SE:   %.4f\n", sd(pop) / sqrt(200)))

# CLT visualization
par(mfrow = c(1, 3))
hist(pop, breaks = 50, main = "Population", xlab = "Allele Freq")
hist(m20, breaks = 20, main = "Means (n=20)", xlab = "Mean")
hist(m200, breaks = 20, main = "Means (n=200)", xlab = "Mean")
