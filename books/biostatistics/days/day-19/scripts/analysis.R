# Day 19: Effect Sizes

data <- read.csv("effect_sizes.csv")
variants <- read.csv("variant_association.csv")

# Cohen's d per gene
cat("=== Cohen's d per Gene ===\n")
for (gene in unique(data$gene)) {
  ctrl <- data$value[data$gene == gene & data$group == "Control"]
  trt <- data$value[data$gene == gene & data$group == "Treatment"]
  pooled_sd <- sqrt(((length(ctrl)-1)*sd(ctrl)^2 + (length(trt)-1)*sd(trt)^2) /
                    (length(ctrl)+length(trt)-2))
  d <- (mean(trt) - mean(ctrl)) / pooled_sd
  p <- t.test(trt, ctrl)$p.value
  cat(sprintf("  %6s: d=%5.2f, p=%.4f, n=%d\n", gene, d, p, length(ctrl)))
}

# Odds ratios
cat("\n=== Variant Odds Ratios ===\n")
for (i in 1:nrow(variants)) {
  row <- variants[i, ]
  or <- (row$case_exposed * row$control_unexposed) /
        (row$case_unexposed * row$control_exposed)
  se <- sqrt(1/row$case_exposed + 1/row$case_unexposed +
             1/row$control_exposed + 1/row$control_unexposed)
  ci_lo <- exp(log(or) - 1.96 * se)
  ci_hi <- exp(log(or) + 1.96 * se)
  cat(sprintf("  %s: OR=%.2f (%.2f-%.2f)\n", row$variant, or, ci_lo, ci_hi))
}
