# Benchmark 10: Large-Scale K-mer Counting
# 21-mer counting on human chromosome 22 (~51 MB).
# R cannot efficiently count 21-mers without external tools.
cat("SKIP: R cannot efficiently count 21-mers on 51 MB genome\n")
quit(status = 1)
