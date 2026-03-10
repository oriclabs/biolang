# Benchmark 2: FASTQ Quality Control
library(ShortRead)

reads <- readFastq("data/reads.fq")
total <- length(reads)
lengths <- width(reads)
quals <- alphabetScore(quality(reads)) / lengths
q30_count <- sum(quals >= 30.0)

cat(sprintf("Total reads: %d\n", total))
cat(sprintf("Q30 rate: %.3f%%\n", q30_count / total * 100.0))
cat(sprintf("Mean length: %.1f\n", mean(lengths)))
cat(sprintf("Min length: %d\n", min(lengths)))
cat(sprintf("Max length: %d\n", max(lengths)))
cat(sprintf("Mean quality: %.2f\n", mean(quals)))
cat(sprintf("Median quality: %.2f\n", median(quals)))
