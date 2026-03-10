lines <- readLines("data/annotations.gff3")
data_lines <- lines[!grepl("^#", lines)]
fields <- strsplit(data_lines, "\t")
types <- sapply(fields, function(x) x[3])
cat(sprintf("Total features: %d\n", length(data_lines)))
cat(sprintf("Genes: %d\n", sum(types == "gene")))
cat(sprintf("Exons: %d\n", sum(types == "exon")))
