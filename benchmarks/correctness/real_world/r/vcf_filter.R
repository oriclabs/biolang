library(jsonlite)
lines <- readLines("real_data/clinvar.vcf")
lines <- lines[!grepl("^#", lines)]
total <- length(lines)
pathogenic_chroms <- c()
for (line in lines) {
  parts <- strsplit(line, "\t")[[1]]
  if (length(parts) >= 8) {
    info <- parts[8]
    fields <- strsplit(info, ";")[[1]]
    for (f in fields) {
      if (startsWith(f, "CLNSIG=")) {
        sig <- sub("CLNSIG=", "", f)
        if (sig == "Pathogenic" || startsWith(sig, "Pathogenic/") || startsWith(sig, "Pathogenic|")) {
          pathogenic_chroms <- c(pathogenic_chroms, parts[1])
        }
        break
      }
    }
  }
}
per_chrom <- as.list(table(pathogenic_chroms))
per_chrom <- lapply(per_chrom, as.integer)
cat(toJSON(list(total_variants = total, pathogenic_count = length(pathogenic_chroms), per_chromosome = per_chrom), auto_unbox = TRUE))
cat("\n")
