library(Biostrings)
records <- readDNAStringSet("data_real/ecoli_genome.fa.gz")
cat(sprintf("Records: %d\n", length(records)))
cat(sprintf("Total bp: %d\n", sum(width(records))))
