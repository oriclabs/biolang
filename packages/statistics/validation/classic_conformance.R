# Regenerates the frozen R constants in
# crates/bl-runtime/tests/stats_classic_conformance.rs
#
# The guided model fitters are pinned to R by stats_model_conformance.rs. The
# classic hypothesis tests are not, even though every one of them is exactly
# computable and so admits a much tighter pin than the directional bounds they
# currently carry -- "p > 0.2 for identical groups" catches a badly wrong
# p-value and not a systematically wrong one.
#
# Run:
#   Rscript packages/statistics/validation/classic_conformance.R

options(digits = 17)
f <- function(x) formatC(x, format = "e", digits = 16)

cat("=== fisher_exact(8, 2, 1, 5) ===\n")
fisher <- fisher.test(matrix(c(8, 2, 1, 5), nrow = 2, byrow = TRUE))
cat("p_two_sided ", f(fisher$p.value), "\n")
# fisher.test reports the conditional maximum likelihood odds ratio. BioLang's
# fisher_exact reports the sample odds ratio, so both are printed and the test
# pins the one it actually computes.
cat("odds_conditional_mle ", f(unname(fisher$estimate)), "\n")
cat("odds_sample          ", f((8 * 5) / (2 * 1)), "\n")

cat("\n=== chi_square, observed vs expected ===\n")
observed <- c(10, 20, 30)
expected <- c(20, 20, 20)
chi <- chisq.test(observed, p = expected / sum(expected))
cat("statistic   ", f(unname(chi$statistic)), "\n")
cat("p_value     ", f(chi$p.value), "\n")
cat("df          ", unname(chi$parameter), "\n")

cat("\n=== wilcoxon rank sum, no continuity correction (Scanpy convention) ===\n")
a <- c(1.2, 2.4, 3.1, 4.8, 5.5)
b <- c(2.0, 3.3, 4.1, 6.2, 7.9)
# R defaults to the exact distribution at this sample size (p = 0.4206).
# BioLang uses the normal approximation without continuity correction, which is
# Scanpy's convention and the one find_all_markers was matched to.
w <- suppressWarnings(wilcox.test(a, b, exact = FALSE, correct = FALSE))
cat("statistic   ", f(unname(w$statistic)), "\n")
cat("p_value     ", f(w$p.value), "\n")

cat("\n=== one-way ANOVA ===\n")
groups <- data.frame(
  value = c(1.0, 2.0, 3.0, 5.0, 6.0, 7.0, 9.0, 10.0, 11.0),
  group = factor(rep(c("a", "b", "c"), each = 3))
)
fit <- summary(aov(value ~ group, data = groups))[[1]]
cat("f_statistic ", f(fit[["F value"]][1]), "\n")
cat("p_value     ", f(fit[["Pr(>F)"]][1]), "\n")
cat("df_between  ", fit[["Df"]][1], "\n")
cat("df_within   ", fit[["Df"]][2], "\n")

cat("\n=== p.adjust ===\n")
p <- c(0.01, 0.04, 0.03, 0.5, 0.2)
cat("bh          ", paste(f(p.adjust(p, "BH")), collapse = " "), "\n")
cat("bonferroni  ", paste(f(p.adjust(p, "bonferroni")), collapse = " "), "\n")

cat("\n=== two-sample t test (Welch, as R's default) ===\n")
# BioLang's ttest is the pooled (Student) form. R defaults to Welch, which
# gives df = 7.399 and p = 0.35287 on this data; var.equal = TRUE is the
# variant being pinned.
tt <- t.test(a, b, var.equal = TRUE)
cat("statistic   ", f(unname(tt$statistic)), "\n")
cat("p_value     ", f(tt$p.value), "\n")
cat("df          ", f(unname(tt$parameter)), "\n")

cat("\n=== pearson correlation ===\n")
x <- c(1.0, 2.0, 3.0, 4.0, 5.0)
y <- c(2.0, 4.1, 5.9, 8.2, 9.8)
ct <- cor.test(x, y, method = "pearson")
cat("estimate    ", f(unname(ct$estimate)), "\n")
cat("p_value     ", f(ct$p.value), "\n")
