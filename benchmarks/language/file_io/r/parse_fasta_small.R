library(Biostrings)
records <- readDNAStringSet("data_real/sarscov2_genome.fa")
cat(sprintf("Records: %d\n", length(records)))
cat(sprintf("Total bp: %d\n", sum(width(records))))
