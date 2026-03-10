# Parse BED, compute total span, count per chromosome, merge overlapping. Output JSON.
library(GenomicRanges)
library(jsonlite)

bed <- read.table("data/promoters.bed", sep = "\t", stringsAsFactors = FALSE)
colnames(bed)[1:3] <- c("chrom", "start", "end")
n <- nrow(bed)
total_span <- sum(bed$end - bed$start)

# Count per chromosome
chrom_counts <- as.list(sort(table(bed$chrom)))
chrom_counts <- lapply(chrom_counts, as.integer)

# Merge overlapping using GenomicRanges
gr <- GRanges(seqnames = bed$chrom, ranges = IRanges(start = bed$start + 1, end = bed$end))
merged <- reduce(gr)
merged_count <- length(merged)

result <- list(
    n_intervals = n,
    total_span = total_span,
    per_chromosome = chrom_counts,
    merged_count = merged_count
)
cat(toJSON(result, auto_unbox = TRUE, pretty = TRUE))
cat("\n")
