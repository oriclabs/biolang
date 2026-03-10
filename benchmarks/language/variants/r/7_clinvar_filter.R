# Benchmark 7: Real-World ClinVar Variant Analysis
# Filter and classify ClinVar variants from NCBI.

total <- 0L
pathogenic_chroms <- character()
pathogenic_refs <- character()
pathogenic_alts <- character()

con <- file("data_real/clinvar_20k.vcf", "r")
while (length(line <- readLines(con, n = 1)) > 0) {
  if (startsWith(line, "#")) next
  total <- total + 1L
  fields <- strsplit(line, "\t")[[1]]
  chrom <- fields[1]
  ref <- fields[4]
  alt <- fields[5]

  # Parse INFO for CLNSIG
  info_parts <- strsplit(fields[8], ";")[[1]]
  clnsig <- ""
  for (part in info_parts) {
    if (startsWith(part, "CLNSIG=")) {
      clnsig <- sub("CLNSIG=", "", part)
      break
    }
  }

  if (clnsig %in% c("Pathogenic", "Likely_pathogenic", "Pathogenic/Likely_pathogenic")) {
    pathogenic_chroms <- c(pathogenic_chroms, chrom)
    pathogenic_refs <- c(pathogenic_refs, ref)
    pathogenic_alts <- c(pathogenic_alts, alt)
  }
}
close(con)

n_path <- length(pathogenic_chroms)
chr1_path <- sum(pathogenic_chroms %in% c("1", "chr1"))
snps <- sum(nchar(pathogenic_refs) == 1 & nchar(pathogenic_alts) == 1)
indels <- n_path - snps

cat(sprintf("Total variants: %d\n", total))
cat(sprintf("Pathogenic/Likely pathogenic: %d\n", n_path))
cat(sprintf("Pathogenic on chr1: %d\n", chr1_path))
cat(sprintf("Pathogenic SNPs: %d\n", snps))
cat(sprintf("Pathogenic Indels: %d\n", indels))
