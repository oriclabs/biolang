"""File I/O: Parse medium FASTA (E. coli, ~4.6 MB)"""
from Bio import SeqIO
records = list(SeqIO.parse("data_real/ecoli_genome.fa", "fasta"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
