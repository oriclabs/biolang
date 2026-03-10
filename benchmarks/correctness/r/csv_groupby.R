# Group CSV by cohort, compute count and mean depth. Output JSON.
library(jsonlite)

samples <- read.csv("data/samples.csv")
groups <- split(samples, samples$cohort)
results <- list()
for (cohort in sort(names(groups))) {
    g <- groups[[cohort]]
    results[[cohort]] <- list(
        count = nrow(g),
        mean_depth = round(mean(g$depth), 6)
    )
}

cat(toJSON(list(groups = results), auto_unbox = TRUE, pretty = TRUE))
cat("\n")
