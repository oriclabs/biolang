"""File I/O: Parse gzipped FASTA (E. coli, ~1.3 MB compressed)"""
import gzip
from Bio import SeqIO

with gzip.open("data_real/ecoli_genome.fa.gz", "rt") as f:
    records = list(SeqIO.parse(f, "fasta"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
