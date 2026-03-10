lines <- readLines("data/variants.vcf")
data_lines <- lines[!grepl("^#", lines)]
cat(sprintf("Records: %d\n", length(data_lines)))
