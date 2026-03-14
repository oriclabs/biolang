# Day 3: Distributions

normal <- read.csv("normal.csv")$value
lognorm <- read.csv("lognormal.csv")$value
poisson <- read.csv("poisson.csv")$value

# Normal: fit and test
cat("=== Normal Distribution ===\n")
cat(sprintf("Mean: %.3f, SD: %.3f\n", mean(normal), sd(normal)))
cat(sprintf("Shapiro-Wilk p: %.4f\n", shapiro.test(normal[1:500])$p.value))
qqnorm(normal, main = "Normal Q-Q Plot")
qqline(normal)

# Log-normal
cat("\n=== Log-Normal Distribution ===\n")
cat(sprintf("Mean: %.3f, SD: %.3f\n", mean(lognorm), sd(lognorm)))
cat(sprintf("Log mean: %.3f, Log SD: %.3f\n", mean(log(lognorm)), sd(log(lognorm))))
shapiro_log <- shapiro.test(log(lognorm)[1:500])
cat(sprintf("Shapiro-Wilk on log(x): p=%.4f\n", shapiro_log$p.value))

# Poisson
cat("\n=== Poisson Distribution ===\n")
cat(sprintf("Mean: %.3f, Var: %.3f\n", mean(poisson), var(poisson)))
cat(sprintf("Var/Mean ratio: %.3f (expect ~1)\n", var(poisson) / mean(poisson)))
