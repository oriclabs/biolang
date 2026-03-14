# Day 17: Survival Analysis
library(survival)

data <- read.csv("breast_cancer_survival.csv")

# Kaplan-Meier
cat("=== Kaplan-Meier by TP53 Status ===\n")
surv_obj <- Surv(data$time_months, data$event)
km_fit <- survfit(surv_obj ~ tp53_mutated, data = data)
print(km_fit)

# Log-rank test
lr <- survdiff(surv_obj ~ tp53_mutated, data = data)
cat(sprintf("Log-rank p-value: %.4f\n", 1 - pchisq(lr$chisq, 1)))

# Cox PH model
cat("\n=== Cox PH Model ===\n")
cox_fit <- coxph(Surv(time_months, event) ~ tp53_mutated + age + factor(stage),
                 data = data)
print(summary(cox_fit))

# Plot
plot(km_fit, col = c("blue", "red"), lwd = 2,
     xlab = "Months", ylab = "Survival Probability",
     main = "Breast Cancer Survival by TP53")
legend("topright", c("WT", "Mutated"), col = c("blue", "red"), lwd = 2)
