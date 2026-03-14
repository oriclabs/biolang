# Day 10: ANOVA

dose <- read.csv("dose_response.csv")
tissue <- read.csv("tissue_expression.csv")

# One-way ANOVA: dose-response
cat("=== One-Way ANOVA (Dose-Response) ===\n")
dose$dose <- factor(dose$dose, levels = c("Placebo", "10mg", "50mg", "200mg"))
fit <- aov(tumor_volume_mm3 ~ dose, data = dose)
print(summary(fit))

cat("\nTukey HSD:\n")
print(TukeyHSD(fit))

# Tissue expression ANOVA
cat("\n=== One-Way ANOVA (Tissue Expression) ===\n")
fit2 <- aov(expression ~ tissue, data = tissue)
print(summary(fit2))

cat("\nTukey HSD:\n")
print(TukeyHSD(fit2))
