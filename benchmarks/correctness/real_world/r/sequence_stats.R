library(Biostrings)
library(jsonlite)
seqs <- readDNAStringSet("real_data/yeast_genome.fa")
lengths <- width(seqs)
total <- sum(lengths)
gc_weighted <- sum(sapply(1:length(seqs), function(i) {
  freq <- alphabetFrequency(seqs[[i]])
  gc <- (freq["G"] + freq["C"]) / sum(freq[c("A","C","G","T")])
  gc * lengths[i]
}))
overall_gc <- gc_weighted / total
sorted_lengths <- sort(lengths, decreasing = TRUE)
cumsum_lengths <- cumsum(sorted_lengths)
n50 <- sorted_lengths[min(which(cumsum_lengths >= total / 2))]
cat(toJSON(list(n_sequences = length(seqs), total_length = total, n50 = as.integer(n50), gc_content = overall_gc), auto_unbox = TRUE))
cat("\n")
