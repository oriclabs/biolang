# Day 24: Bayesian Statistics

# Variant carrier: Beta-Binomial
k <- 3; n <- 1000
a_post <- 1 + k; b_post <- 1 + n - k

cat("=== Bayesian Variant Frequency ===\n")
cat(sprintf("Observed: %d/%d = %.4f\n", k, n, k/n))
cat(sprintf("Posterior: Beta(%d, %d)\n", a_post, b_post))
cat(sprintf("Mean:   %.5f\n", a_post / (a_post + b_post)))
cat(sprintf("95%% CI: (%.5f, %.5f)\n", qbeta(0.025, a_post, b_post),
            qbeta(0.975, a_post, b_post)))

# Informative prior
a2 <- 2 + k; b2 <- 500 + n - k
cat(sprintf("\nInformative prior Beta(2,500):\n"))
cat(sprintf("Posterior: Beta(%d, %d), Mean=%.5f\n", a2, b2, a2/(a2+b2)))

# Plot: prior vs posterior
x <- seq(0, 0.015, length.out = 500)
plot(x, dbeta(x, 1, 1), type = "l", col = "gray", lwd = 2,
     main = "Prior vs Posterior", xlab = "Carrier Frequency", ylab = "Density",
     ylim = c(0, max(dbeta(x, a_post, b_post))))
lines(x, dbeta(x, a_post, b_post), col = "blue", lwd = 2)
lines(x, dbeta(x, a2, b2), col = "red", lwd = 2)
legend("topright", c("Uniform prior", "Posterior (uniform)", "Posterior (informative)"),
       col = c("gray", "blue", "red"), lwd = 2)
