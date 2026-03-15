import json
from Bio import SeqIO
from Bio.SeqUtils import gc_fraction

lengths = []
gc_weighted = 0.0
for record in SeqIO.parse("real_data/yeast_genome.fa", "fasta"):
    l = len(record.seq)
    lengths.append(l)
    gc_weighted += gc_fraction(record.seq) * l

lengths.sort(reverse=True)
total = sum(lengths)
overall_gc = gc_weighted / total

# N50
cumulative = 0
n50 = 0
for l in lengths:
    cumulative += l
    if cumulative >= total / 2:
        n50 = l
        break

print(json.dumps({"n_sequences": len(lengths), "total_length": total, "n50": n50, "gc_content": overall_gc}))
