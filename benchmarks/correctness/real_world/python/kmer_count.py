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
# most_common leaves equal counts in insertion order, which is an artefact
# of how this loop happened to walk the genome. Sort ties by k-mer so the
# result is reproducible by any implementation.
top_10 = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[:10]
print(json.dumps({"sequence_id": record.id, "total_kmers": total, "unique_kmers": len(counts), "top_10": top_10}))
