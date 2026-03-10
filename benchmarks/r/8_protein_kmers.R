# Benchmark 8: Real-World Protein Sequence Analysis
# UniProt reviewed E. coli K-12 proteome.
suppressPackageStartupMessages(library(Biostrings))

seqs <- readAAStringSet("data_real/ecoli_proteome.fa")
lengths <- width(seqs)
total_residues <- sum(lengths)

# N50
sorted_desc <- sort(lengths, decreasing = TRUE)
cumsum_vals <- cumsum(sorted_desc)
n50 <- sorted_desc[which(cumsum_vals >= total_residues / 2)[1]]

cat(sprintf("Proteins: %d\n", length(seqs)))
cat(sprintf("Total residues: %d\n", total_residues))
cat(sprintf("Mean length: %.1f\n", mean(lengths)))
cat(sprintf("Min length: %d\n", min(lengths)))
cat(sprintf("Max length: %d\n", max(lengths)))
cat(sprintf("N50: %d\n", n50))

# Length distribution
short_count <- sum(lengths < 200)
medium_count <- sum(lengths >= 200 & lengths < 500)
long_count <- sum(lengths >= 500)

cat(sprintf("Short (<200 aa): %d\n", short_count))
cat(sprintf("Medium (200-499 aa): %d\n", medium_count))
cat(sprintf("Long (>=500 aa): %d\n", long_count))
