# Group CSV by cohort, compute count and mean age. Output JSON.
library(jsonlite)

meta <- read.csv("data/metadata.csv")
groups <- split(meta, meta$cohort)
results <- list()
for (cohort in sort(names(groups))) {
    g <- groups[[cohort]]
    results[[cohort]] <- list(
        count = nrow(g),
        mean_age = round(mean(g$age), 6)
    )
}

cat(toJSON(list(groups = results), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
