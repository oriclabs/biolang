# Benchmark 9: Real-World Human Chromosome 22 Stats
# Human GRCh38 chr22 from NCBI (~51 MB).
suppressPackageStartupMessages(library(Biostrings))

seqs <- readDNAStringSet("data_real/human_chr22.fa")
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
cat(sprintf("Mean GC: %.4f\n", mean(gc_content)))
cat(sprintf("N50: %d\n", n50))
