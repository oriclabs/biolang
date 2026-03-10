# Benchmark 3: K-mer Counting
# R lacks efficient k=21 counting without external tools (Jellyfish, etc.)
# oligonucleotideFrequency() only works for k <= ~12 (4^k matrix).
# Pure R loops over 27M substrings take hours.
# This benchmark is skipped for R.
cat("SKIP: R cannot efficiently count 21-mers without external tools\n")
quit(status = 1)
