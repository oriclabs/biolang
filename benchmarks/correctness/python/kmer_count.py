"""Count canonical 5-mers from first sequence in FASTA. Output JSON."""
import json
import sys
from Bio import SeqIO

def reverse_complement(seq):
    comp = str.maketrans("ACGT", "TGCA")
    return seq.translate(comp)[::-1]

def canonical(kmer):
    rc = reverse_complement(kmer)
    return min(kmer, rc)

record = next(SeqIO.parse("data/contigs.fa", "fasta"))
seq = str(record.seq).upper()
k = 5
counts = {}
for i in range(len(seq) - k + 1):
    kmer = seq[i:i+k]
    if all(c in "ACGT" for c in kmer):
        ck = canonical(kmer)
        counts[ck] = counts.get(ck, 0) + 1

# Sort by count descending, then alphabetically
top = sorted(counts.items(), key=lambda x: (-x[1], x[0]))[:20]
json.dump({"sequence_id": record.id, "k": k, "total_kmers": sum(counts.values()),
           "unique_canonical": len(counts), "top_20": dict(top)}, sys.stdout, indent=2)
print()
