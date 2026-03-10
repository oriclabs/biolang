# Filter VCF by QUAL>=30, count variants per chromosome. Output JSON.
library(jsonlite)

lines <- readLines("data/variants.vcf")
lines <- lines[!grepl("^#", lines)]
total <- length(lines)

passed_chroms <- c()
for (line in lines) {
    fields <- strsplit(line, "\t")[[1]]
    qual <- suppressWarnings(as.numeric(fields[6]))
    if (!is.na(qual) && qual >= 30) {
        passed_chroms <- c(passed_chroms, fields[1])
    }
}

chrom_counts <- as.list(sort(table(passed_chroms)))
# Convert to integer
chrom_counts <- lapply(chrom_counts, as.integer)

result <- list(
    total_variants = total,
    passed_qual30 = length(passed_chroms),
    per_chromosome = chrom_counts
)
cat(toJSON(result, auto_unbox = TRUE, pretty = TRUE))
cat("\n")
