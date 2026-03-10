library(Biostrings)
records <- readDNAStringSet("data_real/human_chr22.fa")
gc_values <- letterFrequency(records, letters="GC", as.prob=TRUE)[,1]
cat(sprintf("Sequences: %d\n", length(records)))
cat(sprintf("Mean GC: %.4f\n", mean(gc_values)))
cat(sprintf("Min GC: %.4f\n", min(gc_values)))
cat(sprintf("Max GC: %.4f\n", max(gc_values)))
