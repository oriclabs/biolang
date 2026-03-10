library(Biostrings)
records <- readDNAStringSet("data_real/human_chr22.fa.gz")
cat(sprintf("Records: %d\n", length(records)))
cat(sprintf("Total bp: %d\n", sum(width(records))))
