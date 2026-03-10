library(Biostrings)
records <- readDNAStringSet("data/sequences.fa")
filtered <- records[width(records) >= 2000]
writeXStringSet(filtered, "data/filtered_output.fa")
cat(sprintf("Input records: %d\n", length(records)))
cat(sprintf("Filtered records: %d\n", length(filtered)))
cat(sprintf("Written: %d\n", length(filtered)))
