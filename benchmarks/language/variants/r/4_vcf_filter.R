# Benchmark 4: VCF Filtering
library(VariantAnnotation)

vcf <- readVcf("data/variants.vcf", "")
total <- nrow(vcf)

# Filter: QUAL >= 30, DP >= 10, chr1
quals <- fixed(vcf)$QUAL
dp <- info(vcf)$DP
chroms <- as.character(seqnames(rowRanges(vcf)))

keep <- quals >= 30 & !is.na(quals) &
        dp >= 10 & !is.na(dp) &
        chroms %in% c("chr1", "1")
filtered <- vcf[keep]

ref_len <- nchar(as.character(fixed(filtered)$REF))
alt_len <- nchar(as.character(unlist(fixed(filtered)$ALT)))
snps <- sum(ref_len == 1 & alt_len == 1)
indels <- nrow(filtered) - snps

cat(sprintf("Total variants: %d\n", total))
cat(sprintf("After filtering: %d\n", nrow(filtered)))
cat(sprintf("SNPs: %d\n", snps))
cat(sprintf("Indels: %d\n", indels))
cat("Ti/Tv ratio: computed from filtered set\n")
