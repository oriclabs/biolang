"""File I/O: Parse large FASTA (Human chr22, ~51 MB)"""
from Bio import SeqIO
records = list(SeqIO.parse("data_real/human_chr22.fa", "fasta"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
