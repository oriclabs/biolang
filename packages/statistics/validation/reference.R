values <- c(0, 1, 2, 3, 4, 8, 21)
logged <- log1p(values)
x <- c(1, 2, 3, 4, 5, 6)
y <- c(2.1, 3.9, 6.2, 7.8, 10.4, 11.7)
counts <- rbind(c(0, 1, 2), c(0, 10, 20), c(3, 4, 5))

adjusted_skewness <- function(z) {
  n <- length(z)
  n / ((n - 1) * (n - 2)) * sum(((z - mean(z)) / sd(z))^3)
}

fit <- lm(y ~ x)
sample_totals <- rowSums(counts)
fitted_values <- fitted(fit)
residual_values <- residuals(fit)
residual_mse <- sum(residual_values^2) / (length(x) - 2)
qq_expected <- qnorm(ppoints(length(residual_values)))
qq_correlation <- cor(qq_expected, sort(residual_values))
scale_correlation <- cor(fitted_values, abs(residual_values))
curvature_correlation <- cor((x - mean(x))^2, residual_values)
durbin_watson <- sum(diff(residual_values)^2) / sum(residual_values^2)
cook_values <- cooks.distance(fit)
cook_threshold <- 4 / length(x)
screen_x <- 1:8
screen_y <- 2 * screen_x + c(0.1, -0.1, 0.1, -0.1, 0.1, -0.1, 0.1, -0.1)
screen_group <- c("A", "A", "A", "A", "B", "B", "B", "B")
screen_batch <- c("one", "one", "one", "one", "two", "two", "two", "two")
association_table <- table(screen_group, screen_batch)
association_expected <- outer(rowSums(association_table), colSums(association_table)) / sum(association_table)
association_chi_squared <- sum((association_table - association_expected)^2 / association_expected)
association_cramers_v <- sqrt(association_chi_squared / (sum(association_table) * (min(dim(association_table)) - 1)))
association_eta_squared <- sum(tapply(screen_x, screen_group, length) *
  (tapply(screen_x, screen_group, mean) - mean(screen_x))^2) /
  sum((screen_x - mean(screen_x))^2)

distribution_values <- c(0, 0, 0, 1, 1, 2, 4, 9, 15)
distribution_mean <- mean(distribution_values)
distribution_variance_mle <- mean((distribution_values - distribution_mean)^2)
distribution_theta <- distribution_mean^2 / (distribution_variance_mle - distribution_mean)
normal_log_likelihood <- sum(dnorm(
  distribution_values,
  mean = distribution_mean,
  sd = sqrt(distribution_variance_mle),
  log = TRUE
))
poisson_log_likelihood <- sum(dpois(distribution_values, lambda = distribution_mean, log = TRUE))
negative_binomial_log_likelihood <- sum(dnbinom(
  distribution_values,
  size = distribution_theta,
  mu = distribution_mean,
  log = TRUE
))

model_index <- 0:15
model_x1 <- 20 + model_index
model_x2 <- model_index %% 2
model_y <- 1 + 2 * model_x1 + ifelse(model_x2 == 1, 5 + 0.1 * model_x1, 0) +
  (model_index %% 3) * 0.05
multiple_fit <- lm(model_y ~ model_x1 + model_x2 + model_x1:model_x2)
multiple_x <- model.matrix(multiple_fit)
multiple_inverse <- solve(crossprod(multiple_x))
multiple_vif <- vapply(2:ncol(multiple_x), function(j) {
  multiple_inverse[j, j] * sum((multiple_x[, j] - mean(multiple_x[, j]))^2)
}, numeric(1))
multiple_residuals <- residuals(multiple_fit)
multiple_fitted <- fitted(multiple_fit)
multiple_mse <- sum(multiple_residuals^2) / df.residual(multiple_fit)
multiple_qq <- cor(
  qnorm(ppoints(length(multiple_residuals))),
  sort(multiple_residuals)
)
multiple_scale <- cor(multiple_fitted, abs(multiple_residuals))
multiple_dw <- sum(diff(multiple_residuals)^2) / sum(multiple_residuals^2)
multiple_cook <- cooks.distance(multiple_fit)
multiple_leverage <- hatvalues(multiple_fit)

omics_counts <- rbind(
  c(2, 0, 1, 0),
  c(0, 4, 0, 0),
  c(0, 0, 0, 9)
)
omics_feature_means <- colMeans(omics_counts)
omics_feature_variances <- apply(omics_counts, 2, var)
omics_sample_zero_fraction <- rowMeans(omics_counts == 0)

robust_x <- 1:10
robust_y <- c(2, 4.1, 5.9, 8, 10.1, 12, 14, 16.1, 18, 80)
robust_fit <- MASS::rlm(
  robust_y ~ robust_x,
  psi = MASS::psi.huber,
  k2 = 1.345,
  scale.est = "MAD",
  init = "ls",
  maxit = 100,
  acc = 1e-8
)

weighted_values <- c(1, 2, 10)
summary_weights <- c(1, 1, 8)
sum_weights <- sum(summary_weights)
sum_squared_weights <- sum(summary_weights^2)
weighted_center <- sum(weighted_values * summary_weights) / sum_weights
weighted_variance <- sum(summary_weights * (weighted_values - weighted_center)^2) /
  (sum_weights - sum_squared_weights / sum_weights)
weighted_effective_n <- sum_weights^2 / sum_squared_weights

ordered_values <- 1:10
ordered_acf <- as.numeric(acf(ordered_values, plot = FALSE, lag.max = 3)$acf)[2:4]
ordered_ljung_box <- Box.test(ordered_values, lag = 3, type = "Ljung-Box")
ordered_trend <- lm(ordered_values ~ seq_along(ordered_values))

cluster_values <- c(1, 1.2, 5, 5.2, 9, 9.2, 13, 13.2)
cluster_ids <- rep(c("a", "b", "c", "d"), each = 2)
cluster_sizes <- as.numeric(table(cluster_ids))
cluster_means <- tapply(cluster_values, cluster_ids, mean)
cluster_grand_mean <- mean(cluster_values)
cluster_between_ss <- sum(cluster_sizes * (cluster_means - cluster_grand_mean)^2)
cluster_within_ss <- sum((cluster_values - cluster_means[cluster_ids])^2)
cluster_between_ms <- cluster_between_ss / (length(cluster_sizes) - 1)
cluster_within_ms <- cluster_within_ss / (length(cluster_values) - length(cluster_sizes))
cluster_effective_size <- (length(cluster_values) - sum(cluster_sizes^2) / length(cluster_values)) /
  (length(cluster_sizes) - 1)
cluster_icc <- (cluster_between_ms - cluster_within_ms) /
  (cluster_between_ms + (cluster_effective_size - 1) * cluster_within_ms)
cluster_design_effect <- 1 + (mean(cluster_sizes) - 1) * max(cluster_icc, 0)

means_values <- c(1, 2, 4, 8)
means_arithmetic <- mean(means_values)
means_geometric <- exp(mean(log(means_values)))
means_harmonic <- length(means_values) / sum(1 / means_values)
means_trimmed <- mean(means_values, trim = 0.25)
means_rms <- sqrt(mean(means_values^2))

json <- sprintf(
  paste0(
    '{',
    '"descriptive":{"mean":%.17g,"median":%.17g,"variance":%.17g,"sd":%.17g,"q1":%.17g,"q3":%.17g,"mad":%.17g,"skewness":%.17g},',
    '"log1p":{"mean":%.17g,"median":%.17g,"sd":%.17g,"skewness":%.17g},',
    '"relationship":{"pearson":%.17g,"spearman":%.17g,"slope":%.17g,"intercept":%.17g},',
    '"matrix":{"rows":%d,"columns":%d,"zeros":%d,"sample_total_ratio":%.17g,"sample_totals":[%.17g,%.17g,%.17g]},',
    '"linear_diagnostics":{"residual_mse":%.17g,"normal_qq_correlation":%.17g,"scale_correlation":%.17g,"curvature_correlation":%.17g,"durbin_watson":%.17g,"cook_threshold":%.17g,"maximum_cook_distance":%.17g,"cook_review_flags":%d,"standardized_residual_flags":%d},',
    '"associations":{"pearson":%.17g,"spearman":%.17g,"cramers_v":%.17g,"eta_squared":%.17g,"mixed_screening_score":%.17g},',
    '"distribution_clues":{"variance_mean_ratio":%.17g,"expected_poisson_zeros":%.17g,"normal_log_likelihood":%.17g,"normal_aic":%.17g,"poisson_log_likelihood":%.17g,"poisson_aic":%.17g,"negative_binomial_theta":%.17g,"negative_binomial_log_likelihood":%.17g,"negative_binomial_aic":%.17g},',
    '"multiple_linear":{"coef0":%.17g,"coef1":%.17g,"coef2":%.17g,"coef3":%.17g,"r_squared":%.17g,"adjusted_r_squared":%.17g,"residual_mse":%.17g,"maximum_vif":%.17g,"normal_qq_correlation":%.17g,"scale_correlation":%.17g,"durbin_watson":%.17g,"maximum_cook":%.17g,"cook_flags":%d,"leverage_flags":%d},',
    '"omics":{"zero_fraction":%.17g,"sample_total_cv":%.17g,"median_sample_zero_fraction":%.17g,"feature_mean_variance_correlation":%.17g},',
    '"robust_linear":{"intercept":%.17g,"slope":%.17g,"scale":%.17g},',
    '"weighted":{"mean":%.17g,"variance":%.17g,"effective_n":%.17g},',
    '"time_series":{"acf1":%.17g,"acf2":%.17g,"acf3":%.17g,"ljung_box_q":%.17g,"ljung_box_p":%.17g,"trend":%.17g},',
    '"cluster":{"between_ms":%.17g,"within_ms":%.17g,"effective_size":%.17g,"icc":%.17g,"effective_n":%.17g},',
    '"means":{"arithmetic":%.17g,"geometric":%.17g,"harmonic":%.17g,"trimmed":%.17g,"rms":%.17g}',
    '}'
  ),
  mean(values), median(values), var(values), sd(values),
  quantile(values, 0.25, type = 7, names = FALSE),
  quantile(values, 0.75, type = 7, names = FALSE),
  mad(values, constant = 1), adjusted_skewness(values),
  mean(logged), median(logged), sd(logged), adjusted_skewness(logged),
  cor(x, y, method = "pearson"), cor(x, y, method = "spearman"),
  unname(coef(fit)[[2]]), unname(coef(fit)[[1]]),
  nrow(counts), ncol(counts), sum(counts == 0),
  max(sample_totals) / min(sample_totals[sample_totals > 0]),
  sample_totals[[1]], sample_totals[[2]], sample_totals[[3]],
  residual_mse, qq_correlation, scale_correlation, curvature_correlation,
  durbin_watson, cook_threshold, max(cook_values),
  sum(cook_values > cook_threshold), sum(abs(rstandard(fit)) >= 3),
  cor(screen_x, screen_y), cor(screen_x, screen_y, method = "spearman"),
  association_cramers_v, association_eta_squared, sqrt(association_eta_squared),
  distribution_variance_mle / distribution_mean,
  length(distribution_values) * exp(-distribution_mean),
  normal_log_likelihood, 2 * 2 - 2 * normal_log_likelihood,
  poisson_log_likelihood, 2 * 1 - 2 * poisson_log_likelihood,
  distribution_theta, negative_binomial_log_likelihood,
  2 * 2 - 2 * negative_binomial_log_likelihood,
  unname(coef(multiple_fit)[[1]]), unname(coef(multiple_fit)[[2]]),
  unname(coef(multiple_fit)[[3]]), unname(coef(multiple_fit)[[4]]),
  summary(multiple_fit)$r.squared, summary(multiple_fit)$adj.r.squared,
  multiple_mse, max(multiple_vif), multiple_qq, multiple_scale, multiple_dw,
  max(multiple_cook), sum(multiple_cook > 4 / length(model_y)),
  sum(multiple_leverage > 2 * ncol(multiple_x) / length(model_y)),
  mean(omics_counts == 0), sd(rowSums(omics_counts)) / mean(rowSums(omics_counts)),
  median(omics_sample_zero_fraction), cor(omics_feature_means, omics_feature_variances),
  unname(coef(robust_fit)[[1]]), unname(coef(robust_fit)[[2]]), robust_fit$s,
  weighted_center, weighted_variance, weighted_effective_n,
  ordered_acf[[1]], ordered_acf[[2]], ordered_acf[[3]],
  unname(ordered_ljung_box$statistic), ordered_ljung_box$p.value,
  unname(coef(ordered_trend)[[2]]),
  cluster_between_ms, cluster_within_ms, cluster_effective_size, cluster_icc,
  length(cluster_values) / cluster_design_effect,
  means_arithmetic, means_geometric, means_harmonic, means_trimmed, means_rms
)

dir.create("packages/statistics/validation/results", recursive = TRUE, showWarnings = FALSE)
writeLines(json, "packages/statistics/validation/results/r-reference.json", useBytes = TRUE)

# Real-data fixtures are exported from datasets distributed with R/recommended
# packages. They are generated oracle inputs, not vendored project data.
air <- na.omit(airquality[, c("Ozone", "Month")])
names(air) <- c("ozone", "month")
month_sizes <- table(air$month)
air$analysis_weight <- 1 / as.numeric(month_sizes[as.character(air$month)])
write.csv(air, "packages/statistics/validation/results/airquality.csv", row.names = FALSE)

nile <- data.frame(flow = as.numeric(Nile))
write.csv(nile, "packages/statistics/validation/results/nile.csv", row.names = FALSE)

chick <- subset(ChickWeight, Diet == 1, select = c(weight, Time, Chick))
chick$Chick <- as.character(chick$Chick)
names(chick) <- c("weight", "time", "chick")
write.csv(chick, "packages/statistics/validation/results/chickweight.csv", row.names = FALSE)

lung <- survival::lung[, c("time", "status", "age", "sex")]
lung$event <- as.integer(lung$status == 2)
lung$status <- NULL
write.csv(lung, "packages/statistics/validation/results/lung.csv", row.names = FALSE)

air_skew <- adjusted_skewness(air$ozone)
air_log_skew <- adjusted_skewness(log(air$ozone))
air_weight_sum <- sum(air$analysis_weight)
air_weight_sq_sum <- sum(air$analysis_weight^2)
air_weighted_mean <- sum(air$ozone * air$analysis_weight) / air_weight_sum
air_weighted_variance <- sum(air$analysis_weight * (air$ozone - air_weighted_mean)^2) /
  (air_weight_sum - air_weight_sq_sum / air_weight_sum)
air_effective_n <- air_weight_sum^2 / air_weight_sq_sum

nile_acf <- as.numeric(acf(nile$flow, plot = FALSE, lag.max = 3)$acf)[2:4]
nile_ljung <- Box.test(nile$flow, lag = 3, type = "Ljung-Box")
nile_trend <- unname(coef(lm(flow ~ seq_along(flow), data = nile))[[2]])

chick_ids <- chick$chick
chick_sizes <- as.numeric(table(chick_ids))
chick_means <- tapply(chick$weight, chick_ids, mean)
chick_grand_mean <- mean(chick$weight)
chick_between_ss <- sum(chick_sizes * (chick_means - chick_grand_mean)^2)
chick_within_ss <- sum((chick$weight - chick_means[chick_ids])^2)
chick_between_ms <- chick_between_ss / (length(chick_sizes) - 1)
chick_within_ms <- chick_within_ss / (length(chick$weight) - length(chick_sizes))
chick_effective_size <- (length(chick$weight) - sum(chick_sizes^2) / length(chick$weight)) /
  (length(chick_sizes) - 1)
chick_icc <- (chick_between_ms - chick_within_ms) /
  (chick_between_ms + (chick_effective_size - 1) * chick_within_ms)

lung_km <- survival::survfit(survival::Surv(time, event) ~ 1, data = lung)
lung_final_survival <- tail(lung_km$surv, 1)

real_json <- sprintf(
  paste0(
    '{',
    '"airquality":{"observations":%d,"mean":%.17g,"median":%.17g,"sd":%.17g,"skewness":%.17g,"log_skewness":%.17g,"weighted_mean":%.17g,"weighted_variance":%.17g,"effective_n":%.17g},',
    '"nile":{"observations":%d,"acf1":%.17g,"acf2":%.17g,"acf3":%.17g,"ljung_box_q":%.17g,"ljung_box_p":%.17g,"trend":%.17g},',
    '"chickweight":{"observations":%d,"clusters":%d,"between_ms":%.17g,"within_ms":%.17g,"effective_size":%.17g,"icc":%.17g},',
    '"lung":{"observations":%d,"events":%d,"final_survival":%.17g}',
    '}'
  ),
  nrow(air), mean(air$ozone), median(air$ozone), sd(air$ozone), air_skew,
  air_log_skew, air_weighted_mean, air_weighted_variance, air_effective_n,
  nrow(nile), nile_acf[[1]], nile_acf[[2]], nile_acf[[3]],
  unname(nile_ljung$statistic), nile_ljung$p.value, nile_trend,
  nrow(chick), length(unique(chick_ids)), chick_between_ms, chick_within_ms,
  chick_effective_size, chick_icc,
  nrow(lung), sum(lung$event), lung_final_survival
)
writeLines(real_json, "packages/statistics/validation/results/r-real-reference.json", useBytes = TRUE)

glm_binary_data <- data.frame(am = mtcars$am, hp = mtcars$hp, wt = mtcars$wt)
write.csv(glm_binary_data, "packages/statistics/validation/results/mtcars-glm.csv", row.names = FALSE)
glm_binary <- glm(am ~ hp + wt, data = glm_binary_data, family = binomial())
glm_binary_dispersion <- sum(residuals(glm_binary, type = "pearson")^2) /
  df.residual(glm_binary)
glm_binary_brier <- mean((glm_binary_data$am - fitted(glm_binary))^2)

glm_poisson_data <- data.frame(
  breaks = warpbreaks$breaks,
  tension = factor(as.character(warpbreaks$tension), levels = unique(as.character(warpbreaks$tension))),
  wool = factor(as.character(warpbreaks$wool), levels = unique(as.character(warpbreaks$wool)))
)
write.csv(glm_poisson_data, "packages/statistics/validation/results/warpbreaks-glm.csv", row.names = FALSE)
glm_poisson <- glm(breaks ~ tension + wool, data = glm_poisson_data, family = poisson())
glm_poisson_dispersion <- sum(residuals(glm_poisson, type = "pearson")^2) /
  df.residual(glm_poisson)

glm_json <- sprintf(
  paste0(
    '{',
    '"binomial":{"coef0":%.17g,"coef1":%.17g,"coef2":%.17g,"null_deviance":%.17g,"residual_deviance":%.17g,"aic":%.17g,"dispersion":%.17g,"brier":%.17g,"maximum_leverage":%.17g,"maximum_cook":%.17g},',
    '"poisson":{"coef0":%.17g,"coef1":%.17g,"coef2":%.17g,"coef3":%.17g,"null_deviance":%.17g,"residual_deviance":%.17g,"aic":%.17g,"dispersion":%.17g,"expected_zeros":%.17g,"maximum_leverage":%.17g,"maximum_cook":%.17g}',
    '}'
  ),
  coef(glm_binary)[[1]], coef(glm_binary)[[2]], coef(glm_binary)[[3]],
  glm_binary$null.deviance, glm_binary$deviance, AIC(glm_binary),
  glm_binary_dispersion, glm_binary_brier, max(hatvalues(glm_binary)),
  max(cooks.distance(glm_binary)),
  coef(glm_poisson)[[1]], coef(glm_poisson)[[2]], coef(glm_poisson)[[3]],
  coef(glm_poisson)[[4]], glm_poisson$null.deviance, glm_poisson$deviance,
  AIC(glm_poisson), glm_poisson_dispersion,
  sum(exp(-fitted(glm_poisson))), max(hatvalues(glm_poisson)),
  max(cooks.distance(glm_poisson))
)
writeLines(glm_json, "packages/statistics/validation/results/r-glm-reference.json", useBytes = TRUE)

mixed_fit <- nlme::lme(
  weight ~ time,
  random = ~ 1 | chick,
  data = chick,
  method = "REML",
  control = nlme::lmeControl(msMaxIter = 200, returnObject = TRUE)
)
mixed_variances <- as.numeric(nlme::VarCorr(mixed_fit)[, "Variance"])
mixed_random_variance <- mixed_variances[[1]]
mixed_residual_variance <- mixed_variances[[2]]
mixed_json <- sprintf(
  paste0(
    '{',
    '"fixed_intercept":%.17g,"fixed_time":%.17g,',
    '"random_intercept_variance":%.17g,"residual_variance":%.17g,',
    '"icc":%.17g,"clusters":%d,"observations":%d',
    '}'
  ),
  nlme::fixef(mixed_fit)[[1]], nlme::fixef(mixed_fit)[[2]],
  mixed_random_variance, mixed_residual_variance,
  mixed_random_variance / (mixed_random_variance + mixed_residual_variance),
  length(unique(chick$chick)), nrow(chick)
)
writeLines(mixed_json, "packages/statistics/validation/results/r-mixed-reference.json", useBytes = TRUE)

cox_fit <- survival::coxph(
  survival::Surv(time, event) ~ age + sex,
  data = lung,
  ties = "breslow",
  x = TRUE
)
cox_baseline <- survival::basehaz(cox_fit, centered = FALSE)
cox_martingale <- residuals(cox_fit, type = "martingale")
cox_schoenfeld <- residuals(cox_fit, type = "schoenfeld")
cox_event_times <- as.numeric(rownames(cox_schoenfeld))
cox_json <- sprintf(
  paste0(
    '{',
    '"coef_age":%.17g,"coef_sex":%.17g,',
    '"se_age":%.17g,"se_sex":%.17g,',
    '"partial_log_likelihood":%.17g,"likelihood_ratio":%.17g,"aic_partial":%.17g,',
    '"final_baseline_hazard":%.17g,"martingale_sum":%.17g,"martingale_sum_squares":%.17g,',
    '"schoenfeld_age_time_correlation":%.17g,"schoenfeld_sex_time_correlation":%.17g,',
    '"observations":%d,"events":%d',
    '}'
  ),
  coef(cox_fit)[[1]], coef(cox_fit)[[2]],
  sqrt(diag(vcov(cox_fit)))[[1]], sqrt(diag(vcov(cox_fit)))[[2]],
  cox_fit$loglik[[2]], 2 * diff(cox_fit$loglik), AIC(cox_fit),
  tail(cox_baseline$hazard, 1), sum(cox_martingale), sum(cox_martingale^2),
  cor(cox_event_times, cox_schoenfeld[, "age"]),
  cor(cox_event_times, cox_schoenfeld[, "sex"]),
  cox_fit$n, cox_fit$nevent
)
writeLines(cox_json, "packages/statistics/validation/results/r-cox-reference.json", useBytes = TRUE)
