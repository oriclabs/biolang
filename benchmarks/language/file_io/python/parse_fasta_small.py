"""File I/O: Parse small FASTA (SARS-CoV-2, ~30 KB)"""
from Bio import SeqIO
records = list(SeqIO.parse("data_real/sarscov2_genome.fa", "fasta"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
