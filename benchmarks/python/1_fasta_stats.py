"""Benchmark 1: FASTA Statistics"""
from Bio import SeqIO
from statistics import mean, median

records = list(SeqIO.parse("data/sequences.fa", "fasta"))
lengths = [len(r.seq) for r in records]
total_bp = sum(lengths)
gc_values = [(r.seq.count("G") + r.seq.count("C")) / len(r.seq) for r in records]

# N50
sorted_desc = sorted(lengths, reverse=True)
cumsum = 0
n50 = 0
for l in sorted_desc:
    cumsum += l
    if cumsum >= total_bp / 2:
        n50 = l
        break

print(f"Sequences: {len(records)}")
print(f"Total bp: {total_bp}")
print(f"Mean length: {mean(lengths):.1f}")
print(f"Median length: {median(lengths):.1f}")
print(f"Min length: {min(lengths)}")
print(f"Max length: {max(lengths)}")
print(f"Mean GC: {mean(gc_values):.4f}")
print(f"N50: {n50}")
