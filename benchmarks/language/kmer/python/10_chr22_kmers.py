"""Benchmark 10: Large-Scale K-mer Counting
21-mer counting on human chromosome 22 (~51 MB)."""
from Bio import SeqIO
from collections import Counter

records = list(SeqIO.parse("data_real/human_chr22.fa", "fasta"))
k = 21

counts = Counter()
for record in records:
    seq = str(record.seq).upper()
    for i in range(len(seq) - k + 1):
        kmer = seq[i:i + k]
        if "N" not in kmer:
            counts[kmer] += 1

print(f"Distinct {k}-mers: {len(counts)}")
print(f"Top 20 {k}-mers from human chr22:")
for kmer, count in counts.most_common(20):
    print(f"  {kmer}: {count}")
