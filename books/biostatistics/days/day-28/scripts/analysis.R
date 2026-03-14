# Day 28: Capstone — Clinical Trial Analysis
library(survival)

data <- read.csv("clinical_trial.csv")
trt <- data[data$arm == "Treatment", ]
ctrl <- data[data$arm == "Control", ]

# 1. Demographics
cat("=== Demographics ===\n")
cat(sprintf("Treatment: n=%d, age=%.1f +/- %.1f, male=%.0f%%\n",
            nrow(trt), mean(trt$age), sd(trt$age), mean(trt$sex == "M")*100))
cat(sprintf("Control:   n=%d, age=%.1f +/- %.1f, male=%.0f%%\n",
            nrow(ctrl), mean(ctrl$age), sd(ctrl$age), mean(ctrl$sex == "M")*100))

# 2. Response
cat("\n=== RECIST Response ===\n")
data$responder <- data$recist %in% c("CR", "PR")
orr_trt <- mean(trt$recist %in% c("CR", "PR"))
orr_ctrl <- mean(ctrl$recist %in% c("CR", "PR"))
cat(sprintf("ORR Treatment: %.1f%%\n", orr_trt * 100))
cat(sprintf("ORR Control:   %.1f%%\n", orr_ctrl * 100))
print(chisq.test(table(data$arm, data$responder)))

# 3. PFS
cat("\n=== PFS ===\n")
pfs_fit <- survfit(Surv(pfs_months, pfs_event) ~ arm, data = data)
print(pfs_fit)
lr_pfs <- survdiff(Surv(pfs_months, pfs_event) ~ arm, data = data)
cat(sprintf("Log-rank p: %.4f\n", 1 - pchisq(lr_pfs$chisq, 1)))

# 4. Cox PH
cat("\n=== Cox PH (PFS) ===\n")
cox_fit <- coxph(Surv(pfs_months, pfs_event) ~ arm + age + sex + ecog, data = data)
print(summary(cox_fit))

# 5. Safety
cat("\n=== Safety ===\n")
cat(sprintf("Grade 3+ AE: Treatment=%.1f%%, Control=%.1f%%\n",
            mean(trt$grade3_ae)*100, mean(ctrl$grade3_ae)*100))
print(fisher.test(table(data$arm, data$grade3_ae)))

# 6. KM plot
plot(pfs_fit, col = c("red", "blue"), lwd = 2,
     xlab = "Months", ylab = "PFS Probability",
     main = "Progression-Free Survival")
legend("topright", c("Control", "Treatment"), col = c("red", "blue"), lwd = 2)
