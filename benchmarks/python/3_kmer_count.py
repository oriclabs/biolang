"""Benchmark 3: K-mer Counting"""
# Note: counts raw (non-canonical) k-mers.
# BioLang counts canonical (strand-agnostic) k-mers — more work but
# bioinformatically correct.
from Bio import SeqIO
from collections import Counter

records = list(SeqIO.parse("data/sequences.fa", "fasta"))
k = 21

counts = Counter()
for record in records:
    seq = str(record.seq).upper()
    for i in range(len(seq) - k + 1):
        kmer = seq[i:i + k]
        if "N" not in kmer:
            counts[kmer] += 1

print(f"Distinct {k}-mers: {len(counts)}")
print(f"Top 20 {k}-mers:")
for kmer, count in counts.most_common(20):
    print(f"  {kmer}: {count}")
