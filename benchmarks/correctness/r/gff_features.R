# Count features by type from GFF3. Output JSON.
library(jsonlite)

lines <- readLines("data/annotations.gff")
lines <- lines[!grepl("^#", lines) & nchar(trimws(lines)) > 0]
total <- length(lines)

types <- sapply(lines, function(l) strsplit(l, "\t")[[1]][3])
by_type <- as.list(sort(table(types)))
by_type <- lapply(by_type, as.integer)

cat(toJSON(list(total_features = total, by_type = by_type), auto_unbox = TRUE, pretty = TRUE))
cat("\n")
