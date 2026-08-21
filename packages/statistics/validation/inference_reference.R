# Independent R oracle for BioLang's classic inference functions.
#
# This file is executed only by the development validation harness. R is not
# linked, bundled, or invoked by the BioLang statistics package.

options(digits = 17)

a <- c(1.2, 2.4, 3.1, 4.8, 5.5)
b <- c(2.0, 3.3, 4.1, 6.2, 7.9)
one_sample <- c(8.3, 9.1, 10.2, 11.4, 12.0, 9.8)
tiny_sample <- c(1, 2, 4)
paired_before <- c(12.1, 13.5, 11.8, 14.2, 15.0, 13.0)
paired_after <- c(11.7, 12.9, 11.5, 13.1, 14.4, 12.8)
paired_exact_a <- c(11, 8, 13, 6, 15, 4, 17, 2)
paired_exact_b <- rep(10, 8)
ties_a <- c(1, 2, 2, 3, 5)
ties_b <- c(2, 2, 4, 4, 6)
anova_values <- c(1, 2, 3, 5, 6, 7, 9, 10, 11)
anova_group <- factor(rep(c("a", "b", "c"), each = 3))
multi_groups <- list(
  c(1.2, 2.4, 3.1, 4.8, 5.5),
  c(2.0, 3.3, 4.1, 6.2, 7.9, 9.1, 10.3),
  c(0.5, 1.1, 1.8, 2.2)
)
multi_values <- unlist(multi_groups)
multi_group <- factor(rep(seq_along(multi_groups), lengths(multi_groups)))
cor_x <- c(1, 2, 3, 4, 5)
cor_y <- c(2, 4.1, 5.9, 8.2, 9.8)
p_raw <- c(0.01, 0.04, 0.03, 0.5, 0.2)
p_boundary <- c(0, 0.01, 0.01, 1)

pooled <- t.test(a, b, var.equal = TRUE)
welch <- t.test(a, b)
one <- t.test(one_sample, mu = 10)
tiny <- t.test(tiny_sample, mu = 2)
paired <- t.test(paired_before, paired_after, paired = TRUE)
wilcoxon_normal <- suppressWarnings(wilcox.test(a, b, exact = FALSE, correct = FALSE))
wilcoxon_continuity <- suppressWarnings(wilcox.test(a, b, exact = FALSE, correct = TRUE))
wilcoxon_default <- suppressWarnings(wilcox.test(a, b))
wilcoxon_ties <- suppressWarnings(wilcox.test(ties_a, ties_b, exact = FALSE, correct = FALSE))
paired_rank_normal <- suppressWarnings(wilcox.test(paired_before, paired_after, paired = TRUE, exact = FALSE, correct = FALSE))
paired_rank_continuity <- suppressWarnings(wilcox.test(paired_before, paired_after, paired = TRUE, exact = FALSE, correct = TRUE))
paired_rank_exact <- wilcox.test(paired_exact_a, paired_exact_b, paired = TRUE, exact = TRUE)
anova_fit <- summary(aov(anova_values ~ anova_group))[[1]]
multi_classical <- oneway.test(multi_values ~ multi_group, var.equal = TRUE)
multi_welch <- oneway.test(multi_values ~ multi_group, var.equal = FALSE)
multi_kruskal <- kruskal.test(multi_values ~ multi_group)
multi_tukey <- TukeyHSD(aov(multi_values ~ multi_group))[[1]]
multi_pair_tests <- list(
  t.test(multi_groups[[1]], multi_groups[[2]]),
  t.test(multi_groups[[1]], multi_groups[[3]]),
  t.test(multi_groups[[2]], multi_groups[[3]])
)
multi_pair_raw <- vapply(multi_pair_tests, function(test) test$p.value, numeric(1))
multi_pair_holm <- p.adjust(multi_pair_raw, "holm")
multi_ss_between <- sum(lengths(multi_groups) * (vapply(multi_groups, mean, numeric(1)) - mean(multi_values))^2)
multi_ss_within <- sum(vapply(multi_groups, function(group) sum((group - mean(group))^2), numeric(1)))
multi_ss_total <- multi_ss_between + multi_ss_within
multi_ms_within <- multi_ss_within / (length(multi_values) - length(multi_groups))
multi_eta_squared <- multi_ss_between / multi_ss_total
multi_omega_squared <- (multi_ss_between - (length(multi_groups) - 1) * multi_ms_within) /
  (multi_ss_total + multi_ms_within)
multi_epsilon_squared <- (unname(multi_kruskal$statistic) - length(multi_groups) + 1) /
  (length(multi_values) - length(multi_groups))
fisher <- fisher.test(matrix(c(8, 2, 1, 5), nrow = 2, byrow = TRUE))
chi <- chisq.test(c(10, 20, 30), p = c(20, 20, 20) / 60)

# BioLang deliberately reports the unconditional sample odds ratio. R's
# fisher.test reports a conditional maximum-likelihood estimate instead.
fisher_sample_odds <- (8 * 5) / (2 * 1)
fisher_wald_se <- sqrt(1 / 8 + 1 / 2 + 1 / 1 + 1 / 5)
fisher_wald_ci <- exp(log(fisher_sample_odds) + c(-1, 1) * qnorm(0.975) * fisher_wald_se)

pooled_variance <- ((length(a) - 1) * var(a) + (length(b) - 1) * var(b)) /
  (length(a) + length(b) - 2)
cohens_d <- (mean(a) - mean(b)) / sqrt(pooled_variance)
hedges_g <- cohens_d * (1 - 3 / (4 * (length(a) + length(b)) - 9))
one_effect <- (mean(one_sample) - 10) / sd(one_sample)
paired_effect <- mean(paired_before - paired_after) / sd(paired_before - paired_after)
rank_biserial <- 2 * unname(wilcoxon_normal$statistic) / (length(a) * length(b)) - 1
paired_rank_total <- sum(rank(abs(paired_before - paired_after)))
paired_rank_biserial <- 2 * unname(paired_rank_normal$statistic) / paired_rank_total - 1

json_numbers <- function(values) paste(sprintf("%.17g", values), collapse = ",")
json <- sprintf(
  paste0(
    '{',
    '"ttest_pooled":{"statistic":%.17g,"p_value":%.17g,"df":%.17g,"standard_error":%.17g,"confidence_lower":%.17g,"confidence_upper":%.17g,"cohens_d":%.17g,"hedges_g":%.17g},',
    '"ttest_r_default":{"statistic":%.17g,"p_value":%.17g,"df":%.17g,"standard_error":%.17g,"confidence_lower":%.17g,"confidence_upper":%.17g},',
    '"ttest_one":{"statistic":%.17g,"p_value":%.17g,"df":%.17g,"confidence_lower":%.17g,"confidence_upper":%.17g,"cohens_d":%.17g},',
    '"ttest_tiny":{"statistic":%.17g,"p_value":%.17g,"df":%.17g},',
    '"ttest_paired":{"statistic":%.17g,"p_value":%.17g,"df":%.17g,"confidence_lower":%.17g,"confidence_upper":%.17g,"cohens_dz":%.17g},',
    '"wilcoxon_normal":{"statistic":%.17g,"p_value":%.17g,"rank_biserial":%.17g},',
    '"wilcoxon_continuity":{"statistic":%.17g,"p_value":%.17g},',
    '"wilcoxon_r_default":{"statistic":%.17g,"p_value":%.17g},',
    '"wilcoxon_ties":{"statistic":%.17g,"p_value":%.17g},',
    '"wilcoxon_paired_normal":{"statistic":%.17g,"p_value":%.17g,"rank_biserial":%.17g},',
    '"wilcoxon_paired_continuity":{"statistic":%.17g,"p_value":%.17g},',
    '"wilcoxon_paired_exact":{"statistic":%.17g,"p_value":%.17g},',
    '"anova":{"f_statistic":%.17g,"p_value":%.17g,"df_between":%.17g,"df_within":%.17g},',
    '"anova_welch":{"f_statistic":%.17g,"p_value":%.17g,"df_between":%.17g,"df_within":%.17g,"ss_between":%.17g,"ss_within":%.17g,"ss_total":%.17g,"eta_squared":%.17g,"omega_squared":%.17g},',
    '"kruskal_wallis":{"h_statistic":%.17g,"p_value":%.17g,"df":%.17g,"epsilon_squared":%.17g},',
    '"tukey_hsd":{"critical_value":%.17g,"mean_square_within":%.17g,"mean_differences":[%s],"p_values":[%s],"confidence_lower":[%s],"confidence_upper":[%s]},',
    '"pairwise_welch_holm":{"raw_p_values":[%s],"adjusted_p_values":[%s]},',
    '"fisher":{"p_value":%.17g,"sample_odds_ratio":%.17g,"r_conditional_odds_ratio":%.17g,"wald_lower":%.17g,"wald_upper":%.17g},',
    '"chi_square":{"statistic":%.17g,"p_value":%.17g,"df":%.17g},',
    '"correlation":{"pearson":%.17g},',
    '"p_adjust":{"bh":[%s],"bonferroni":[%s],"holm":[%s]},',
    '"p_adjust_boundary":{"bh":[%s],"bonferroni":[%s],"holm":[%s]}',
    '}'
  ),
  unname(pooled$statistic), pooled$p.value, unname(pooled$parameter),
  sqrt(pooled_variance * (1 / length(a) + 1 / length(b))), pooled$conf.int[[1]], pooled$conf.int[[2]],
  cohens_d, hedges_g,
  unname(welch$statistic), welch$p.value, unname(welch$parameter),
  sqrt(var(a) / length(a) + var(b) / length(b)), welch$conf.int[[1]], welch$conf.int[[2]],
  unname(one$statistic), one$p.value, unname(one$parameter), one$conf.int[[1]] - 10, one$conf.int[[2]] - 10, one_effect,
  unname(tiny$statistic), tiny$p.value, unname(tiny$parameter),
  unname(paired$statistic), paired$p.value, unname(paired$parameter), paired$conf.int[[1]], paired$conf.int[[2]], paired_effect,
  unname(wilcoxon_normal$statistic), wilcoxon_normal$p.value, rank_biserial,
  unname(wilcoxon_continuity$statistic), wilcoxon_continuity$p.value,
  unname(wilcoxon_default$statistic), wilcoxon_default$p.value,
  unname(wilcoxon_ties$statistic), wilcoxon_ties$p.value,
  unname(paired_rank_normal$statistic), paired_rank_normal$p.value, paired_rank_biserial,
  unname(paired_rank_continuity$statistic), paired_rank_continuity$p.value,
  unname(paired_rank_exact$statistic), paired_rank_exact$p.value,
  anova_fit[["F value"]][1], anova_fit[["Pr(>F)"]][1],
  anova_fit[["Df"]][1], anova_fit[["Df"]][2],
  unname(multi_welch$statistic), multi_welch$p.value,
  unname(multi_welch$parameter[[1]]), unname(multi_welch$parameter[[2]]),
  multi_ss_between, multi_ss_within, multi_ss_total, multi_eta_squared, multi_omega_squared,
  unname(multi_kruskal$statistic), multi_kruskal$p.value, unname(multi_kruskal$parameter), multi_epsilon_squared,
  qtukey(0.95, length(multi_groups), length(multi_values) - length(multi_groups)), multi_ms_within,
  json_numbers(-multi_tukey[, "diff"]),
  json_numbers(multi_tukey[, "p adj"]),
  json_numbers(-multi_tukey[, "upr"]),
  json_numbers(-multi_tukey[, "lwr"]),
  json_numbers(multi_pair_raw), json_numbers(multi_pair_holm),
  fisher$p.value, fisher_sample_odds, unname(fisher$estimate), fisher_wald_ci[[1]], fisher_wald_ci[[2]],
  unname(chi$statistic), chi$p.value, unname(chi$parameter),
  cor(cor_x, cor_y),
  json_numbers(p.adjust(p_raw, "BH")),
  json_numbers(p.adjust(p_raw, "bonferroni")),
  json_numbers(p.adjust(p_raw, "holm")),
  json_numbers(p.adjust(p_boundary, "BH")),
  json_numbers(p.adjust(p_boundary, "bonferroni")),
  json_numbers(p.adjust(p_boundary, "holm"))
)

writeLines(
  json,
  "packages/statistics/validation/results/r-inference-reference.json",
  useBytes = TRUE
)
