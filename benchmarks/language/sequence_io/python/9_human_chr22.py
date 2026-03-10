"""Benchmark 9: Real-World Human Chromosome 22 Stats
Human GRCh38 chr22 from NCBI (~51 MB)."""
from Bio import SeqIO
from statistics import mean

records = list(SeqIO.parse("data_real/human_chr22.fa", "fasta"))
lengths = [len(r.seq) for r in records]
total_bp = sum(lengths)
gc_values = [(r.seq.count("G") + r.seq.count("C")) / len(r.seq) if len(r.seq) > 0 else 0.0
             for r in records]

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
print(f"Mean GC: {mean(gc_values):.4f}")
print(f"N50: {n50}")
