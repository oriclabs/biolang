# Regenerates the frozen R constants in
# crates/bl-runtime/tests/stats_model_conformance.rs
#
# The main oracle (reference.R + run.ps1) compares 147 metrics against R but
# needs R installed and is run by hand, so it cannot guard a refactor. This
# script covers the same four fitters using the small fixtures that are
# already committed in packages/statistics/tests/exploration.bl, so the
# resulting constants can be pasted into an ordinary `cargo test` that runs
# everywhere without R and without redistributing any R dataset.
#
# Run:
#   Rscript packages/statistics/validation/model_conformance.R
#
# Then copy each printed constant into the Rust test. Values are printed with
# 17 significant digits so a f64 round trip is exact.

options(digits = 17)
f <- function(x) formatC(x, format = "e", digits = 16)

# `hatvalues()` reads the QR that glm.fit built during the final IRLS
# iteration, so its weights belong to the previous coefficient vector, not the
# converged one. BioLang recomputes the hat matrix at the converged
# coefficients, which differs from `hatvalues()` in the sixth significant digit
# on a Poisson fit -- the working weights are exp(eta) there and move more
# between the last two iterations than binomial's mu(1-mu) does.
#
# Neither is wrong; they are different quantities. This helper reports the
# recomputed one so the frozen test can pin BioLang's intended definition
# tightly instead of hiding a real difference under a loose tolerance.
max_leverage_at_beta <- function(fit) {
  x <- model.matrix(fit)
  weight <- as.vector(fit$family$variance(fit$family$linkinv(x %*% coef(fit))))
  weighted <- x * sqrt(weight)
  max(diag(weighted %*% solve(t(weighted) %*% weighted) %*% t(weighted)))
}

# A deliberately overlapping fixture. The 12-row fixture in
# packages/statistics/tests/exploration.bl is perfectly separable -- R reports
# "algorithm did not converge" on it -- so it pins the convergence flag rather
# than any coefficient. This one has a finite MLE and pins the numbers.
cat("=== binomial GLM ===\n")
binomial_data <- data.frame(
  age = c(20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80),
  marker = c(
    0.2, 1.4, 0.4, 1.2, 0.7, 0.3, 1.1, 2.0,
    0.5, 2.4, 1.9, 0.8, 2.2, 1.0, 2.6, 1.3
  ),
  y = c(0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 1)
)
binomial_fit <- glm(y ~ age + marker, data = binomial_data, family = binomial())
cat("converged      ", binomial_fit$converged, "\n")
cat("coef_intercept ", f(coef(binomial_fit)[[1]]), "\n")
cat("coef_age       ", f(coef(binomial_fit)[[2]]), "\n")
cat("coef_marker    ", f(coef(binomial_fit)[[3]]), "\n")
cat("null_deviance  ", f(binomial_fit$null.deviance), "\n")
cat("residual_dev   ", f(binomial_fit$deviance), "\n")
cat("aic            ", f(AIC(binomial_fit)), "\n")
cat("max_leverage   ", f(max(hatvalues(binomial_fit))), "\n")
cat("max_lev_at_beta", f(max_leverage_at_beta(binomial_fit)), "\n")
cat("max_cook       ", f(max(cooks.distance(binomial_fit))), "\n")
cat("brier          ", f(mean((binomial_data$y - fitted(binomial_fit))^2)), "\n")

cat("\n=== poisson GLM ===\n")
poisson_data <- data.frame(
  exposure = 1:10,
  y = c(0, 1, 1, 2, 2, 4, 3, 5, 7, 8)
)
poisson_fit <- glm(y ~ exposure, data = poisson_data, family = poisson())
cat("converged      ", poisson_fit$converged, "\n")
cat("coef_intercept ", f(coef(poisson_fit)[[1]]), "\n")
cat("coef_exposure  ", f(coef(poisson_fit)[[2]]), "\n")
cat("null_deviance  ", f(poisson_fit$null.deviance), "\n")
cat("residual_dev   ", f(poisson_fit$deviance), "\n")
cat("aic            ", f(AIC(poisson_fit)), "\n")
cat("expected_zeros ", f(sum(exp(-fitted(poisson_fit)))), "\n")
cat("max_leverage   ", f(max(hatvalues(poisson_fit))), "\n")
cat("max_lev_at_beta", f(max_leverage_at_beta(poisson_fit)), "\n")

# The 18-row fixture in exploration.bl leaves a residual variance near 0.017
# against a random-intercept variance near 17.9, which puts the REML profile
# almost on its boundary. This fixture keeps real within-cluster spread so the
# optimum is interior and the pinned variance components are stable.
cat("\n=== random intercept (REML) ===\n")
mixed_data <- data.frame(
  time = rep(c(0, 1, 2, 3), times = 6),
  weight = c(
    10.4, 12.1, 14.6, 15.9, 15.0, 17.9, 18.4, 21.2,
    6.3, 8.1, 9.4, 12.2, 13.0, 15.8, 16.5, 19.4,
    8.7, 10.2, 12.4, 13.6, 17.0, 19.5, 20.6, 23.5
  ),
  cluster = rep(c("a", "b", "c", "d", "e", "f"), each = 4)
)
mixed_fit <- nlme::lme(
  weight ~ time,
  random = ~ 1 | cluster,
  data = mixed_data,
  method = "REML",
  control = nlme::lmeControl(msMaxIter = 200, returnObject = TRUE)
)
mixed_var <- as.numeric(nlme::VarCorr(mixed_fit)[, "Variance"])
cat("fixed_intercept", f(nlme::fixef(mixed_fit)[[1]]), "\n")
cat("fixed_time     ", f(nlme::fixef(mixed_fit)[[2]]), "\n")
cat("random_var     ", f(mixed_var[[1]]), "\n")
cat("residual_var   ", f(mixed_var[[2]]), "\n")
cat("icc            ", f(mixed_var[[1]] / sum(mixed_var)), "\n")

cat("\n=== Cox (breslow) ===\n")
cox_data <- data.frame(
  time = c(5, 8, 9, 12, 15, 18, 20, 23, 26, 30, 32, 35),
  event = c(1, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1),
  age = c(51, 62, 57, 70, 45, 66, 54, 73, 49, 61, 58, 68),
  treatment = c(0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0)
)
cox_fit <- survival::coxph(
  survival::Surv(time, event) ~ age + treatment,
  data = cox_data,
  ties = "breslow"
)
cox_summary <- summary(cox_fit)
cat("coef_age       ", f(coef(cox_fit)[[1]]), "\n")
cat("coef_treatment ", f(coef(cox_fit)[[2]]), "\n")
cat("se_age         ", f(cox_summary$coefficients[1, "se(coef)"]), "\n")
cat("se_treatment   ", f(cox_summary$coefficients[2, "se(coef)"]), "\n")
cat("partial_loglik ", f(cox_fit$loglik[[2]]), "\n")
cat("null_loglik    ", f(cox_fit$loglik[[1]]), "\n")
cat("likelihood_rat ", f(2 * (cox_fit$loglik[[2]] - cox_fit$loglik[[1]])), "\n")
cat("martingale_ss  ", f(sum(residuals(cox_fit, type = "martingale")^2)), "\n")

# The separable fixture from exploration.bl. Only the convergence flag is
# pinned from this one: a separated logistic fit has no finite MLE, so its
# coefficients depend entirely on where the iteration stops.
cat("\n=== binomial GLM, separable fixture ===\n")
separable_data <- data.frame(
  age = c(20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64),
  marker = c(0.2, 0.8, 0.4, 1.2, 0.7, 1.6, 1.1, 2.0, 1.5, 2.4, 1.9, 2.8),
  y = c(0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1)
)
separable_fit <- suppressWarnings(
  glm(y ~ age + marker, data = separable_data, family = binomial())
)
cat("converged      ", separable_fit$converged, "\n")
cat("residual_dev   ", f(separable_fit$deviance), "\n")
