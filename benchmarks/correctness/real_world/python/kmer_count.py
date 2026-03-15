import json
from Bio import SeqIO
from collections import Counter

record = next(SeqIO.parse("real_data/ecoli_genome.fa", "fasta"))
seq = str(record.seq)[:50000].upper()
k = 5
counts = Counter()
for i in range(len(seq) - k + 1):
    kmer = seq[i:i+k]
    if all(c in "ACGT" for c in kmer):
        rc = kmer.translate(str.maketrans("ACGT", "TGCA"))[::-1]
        canonical = min(kmer, rc)
        counts[canonical] += 1
total = sum(counts.values())
top_10 = counts.most_common(10)
print(json.dumps({"sequence_id": record.id, "total_kmers": total, "unique_kmers": len(counts), "top_10": top_10}))
