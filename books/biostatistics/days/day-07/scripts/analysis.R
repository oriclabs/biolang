# Day 7: Hypothesis Testing

bio <- read.csv("biomarker.csv")
null_z <- read.csv("null_simulation.csv")$z_statistic

ad <- bio$biomarker_ngml[bio$group == "AD"]
ctrl <- bio$biomarker_ngml[bio$group == "Control"]

# Manual z-test
mean_diff <- mean(ad) - mean(ctrl)
se <- sqrt(var(ad)/length(ad) + var(ctrl)/length(ctrl))
z_obs <- mean_diff / se
p_val <- 2 * pnorm(-abs(z_obs))

cat("=== Two-Sample Z-Test ===\n")
cat(sprintf("AD mean:      %.2f\n", mean(ad)))
cat(sprintf("Control mean: %.2f\n", mean(ctrl)))
cat(sprintf("Z-statistic:  %.3f\n", z_obs))
cat(sprintf("P-value:      %.4f\n", p_val))
cat(sprintf("Reject H0? %s\n", ifelse(p_val < 0.05, "Yes", "No")))

# Null simulation
exceed <- mean(abs(null_z) >= abs(z_obs))
cat(sprintf("\nNull sim: %.3f of z-stats exceed |%.2f|\n", exceed, z_obs))

# Visualization
hist(null_z, breaks = 50, main = "Null Distribution",
     xlab = "Z-statistic", col = "lightblue")
abline(v = c(-z_obs, z_obs), col = "red", lwd = 2)
