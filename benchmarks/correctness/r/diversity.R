# Shannon and Simpson diversity over GFF feature types. Output JSON.
library(jsonlite)

lines <- readLines("data/annotations.gff3")
lines <- lines[!startsWith(lines, "#") & nzchar(trimws(lines))]
types <- sapply(strsplit(lines, "\t", fixed = TRUE), function(p) if (length(p) >= 3) p[3] else NA)
types <- types[!is.na(types)]

counts <- table(types)
total <- sum(counts)
ps <- as.numeric(counts[order(names(counts))]) / total
shannon <- -sum(ps * log(ps))
simpson <- 1 - sum(ps^2)

cat(toJSON(list(n_categories = length(ps), n_observations = as.integer(total),
                shannon = round(shannon, 9), simpson = round(simpson, 9)),
           auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
