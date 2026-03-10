# Benchmark 1: FASTA Statistics
library(Biostrings)

seqs <- readDNAStringSet("data/sequences.fa")
lengths <- width(seqs)
total_bp <- sum(lengths)
gc_values <- letterFrequency(seqs, "GC", as.prob = TRUE)[, 1]

# N50
sorted_desc <- sort(lengths, decreasing = TRUE)
cumsum_vals <- cumsum(sorted_desc)
n50 <- sorted_desc[which(cumsum_vals >= total_bp / 2)[1]]

cat(sprintf("Sequences: %d\n", length(seqs)))
cat(sprintf("Total bp: %d\n", total_bp))
cat(sprintf("Mean length: %.1f\n", mean(lengths)))
cat(sprintf("Median length: %.1f\n", median(lengths)))
cat(sprintf("Min length: %d\n", min(lengths)))
cat(sprintf("Max length: %d\n", max(lengths)))
cat(sprintf("Mean GC: %.4f\n", mean(gc_values)))
cat(sprintf("N50: %d\n", n50))
