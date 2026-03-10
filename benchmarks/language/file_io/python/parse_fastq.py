"""File I/O: Parse FASTQ (100K reads, ~26 MB)"""
from Bio import SeqIO
records = list(SeqIO.parse("data/reads.fq", "fastq"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
