library(jsonlite)
data <- read.csv("real_data/clinvar_variants.csv", stringsAsFactors = FALSE)
groups <- split(data, data$clnsig)
result <- list()
for (sig in names(groups)) {
  g <- groups[[sig]]
  result[[sig]] <- list(count = nrow(g), mean_var_len = mean(g$var_len))
}
cat(toJSON(list(groups = result), auto_unbox = TRUE))
cat("\n")
