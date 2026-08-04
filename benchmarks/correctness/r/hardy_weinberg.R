# Hardy-Weinberg expectations from observed genotype counts. Output JSON.
library(jsonlite)

cohorts <- list(
    list(name = "cohort_1", aa = 1469, ab = 138, bb = 5),
    list(name = "cohort_2", aa = 900,  ab = 850, bb = 250),
    list(name = "cohort_3", aa = 320,  ab = 480, bb = 200)
)

results <- lapply(cohorts, function(c) {
    n <- c$aa + c$ab + c$bb
    p <- (2 * c$aa + c$ab) / (2 * n)
    q <- 1 - p
    list(name = c$name, n = as.integer(n),
         p = round(p, 9), q = round(q, 9),
         exp_aa = round(p * p * n, 6),
         exp_ab = round(2 * p * q * n, 6),
         exp_bb = round(q * q * n, 6))
})

cat(toJSON(list(cohorts = results), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
