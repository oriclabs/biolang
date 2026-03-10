# Benchmark 6: Real-World Genome Assembly Stats
# E. coli K-12 MG1655 complete genome from NCBI RefSeq.
suppressPackageStartupMessages(library(Biostrings))

seqs <- readDNAStringSet("data_real/ecoli_genome.fa")
lengths <- width(seqs)
total_bp <- sum(lengths)
gc <- letterFrequency(seqs, letters = c("G", "C"))
gc_content <- rowSums(gc) / lengths

# N50
sorted_desc <- sort(lengths, decreasing = TRUE)
cumsum_vals <- cumsum(sorted_desc)
n50 <- sorted_desc[which(cumsum_vals >= total_bp / 2)[1]]

cat(sprintf("Sequences: %d\n", length(seqs)))
cat(sprintf("Total bp: %d\n", total_bp))
cat(sprintf("Mean length: %.1f\n", mean(lengths)))
cat(sprintf("Min length: %d\n", min(lengths)))
cat(sprintf("Max length: %d\n", max(lengths)))
cat(sprintf("Mean GC: %.4f\n", mean(gc_content)))
cat(sprintf("N50: %d\n", n50))
