"""Compute N50, total length, sequence count, mean GC from FASTA. Output JSON."""
import json
import sys
from Bio import SeqIO

lengths = []
gc_values = []
for record in SeqIO.parse("data/contigs.fa", "fasta"):
    seq = str(record.seq).upper()
    lengths.append(len(seq))
    gc = (seq.count("G") + seq.count("C")) / len(seq) if len(seq) > 0 else 0.0
    gc_values.append(gc)

# N50 calculation
lengths_sorted = sorted(lengths, reverse=True)
total_len = sum(lengths_sorted)
cumsum = 0
n50 = 0
for l in lengths_sorted:
    cumsum += l
    if cumsum >= total_len / 2:
        n50 = l
        break

mean_gc = round(sum(gc_values) / len(gc_values), 6) if gc_values else 0.0

json.dump({
    "n_sequences": len(lengths),
    "total_length": total_len,
    "n50": n50,
    "min_length": min(lengths),
    "max_length": max(lengths),
    "mean_gc": mean_gc
}, sys.stdout, indent=2)
print()
