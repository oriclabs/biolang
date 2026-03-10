"""GC Content at Scale: compute GC for every sequence in a large FASTA"""
from Bio import SeqIO
from Bio.SeqUtils import gc_fraction

records = list(SeqIO.parse("data_real/human_chr22.fa", "fasta"))
gc_values = [gc_fraction(r.seq) for r in records]
print(f"Sequences: {len(records)}")
print(f"Mean GC: {round(sum(gc_values) / len(gc_values), 4)}")
print(f"Min GC: {round(min(gc_values), 4)}")
print(f"Max GC: {round(max(gc_values), 4)}")
